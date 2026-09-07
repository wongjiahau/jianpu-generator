import { resolve_selection_range } from '../jianpuWasm'
import type { LyricSpan, NoteSpan } from '../types'
import type { ClickableElementId } from './clickableElementId'
import {
  anyClickableElementIdAtPoint,
  measureClickableElementId,
} from './previewClickableElementIdBuilders'
import {
  applyPersistedLyricHighlights,
  applyPersistedNoteHighlights,
} from './previewRangeHighlights'
import type { PreviewAnchorState } from './previewAnchorState'
import {
  applyLyricRangeSelection,
  applyNoteRangeSelection,
} from './previewRangeSelection'
import {
  getMeasureAtPoint,
  type LyricCell,
  lyricCellsInMeasureRange,
  type NoteCell,
  noteCellsInMeasureRange,
  systemRangeContainingMeasure,
} from './previewSelection'
import type { ResolvedSelection } from './previewSelectionResolver'

/** Everything a mode's own resolver needs beyond `anchorState`/`point`/
 * `currentIdHint` themselves — shared by `resolveMeasureSelection`,
 * `resolveNoteSelection`, and `resolveLyricSelection` below (and their
 * label-mode siblings in `previewSelectionResolveLabelModes.ts`), split out
 * of `resolveSelection`'s own body once its per-mode branches grew too long
 * for one function. */
export interface ResolveModeArgs {
  container: HTMLDivElement | null
  point: { x: number; y: number } | undefined
  currentIdHint: ClickableElementId | undefined
  noteSpans: NoteSpan[]
  lyricSpans: LyricSpan[]
}

type MeasureAnchorState = Extract<
  NonNullable<PreviewAnchorState>,
  { mode: 'measure' }
>
type BarNumberSystemAnchorState = Extract<
  NonNullable<PreviewAnchorState>,
  { mode: 'bar-number-system' }
>
type NoteAnchorState = Extract<NonNullable<PreviewAnchorState>, { mode: 'note' }>
type LyricAnchorState = Extract<NonNullable<PreviewAnchorState>, { mode: 'lyric' }>

export function resolveMeasureSelection(
  anchorState: MeasureAnchorState,
  { container, point, currentIdHint, noteSpans, lyricSpans }: ResolveModeArgs,
): ResolvedSelection {
  const finalRange = point
    ? (getMeasureAtPoint(point.x, point.y) ?? anchorState.current)
    : anchorState.anchor
  const response = resolve_selection_range(
    noteSpans,
    lyricSpans,
    anchorState.anchorId,
    currentIdHint ?? measureClickableElementId(finalRange),
  )
  // `Measure ↔ Measure` is fully ID-based now (see
  // `resolve_selection_range_response` in `selection_range.rs`), so this
  // never actually returns 'err' for 'measure' mode — the pixel-based
  // fallback below is dead code kept only as a safety net until the wasm
  // side is proven out further.
  let noteCells: NoteCell[]
  let lyricCells: LyricCell[]
  if (response.status === 'ok') {
    noteCells = response.note_cells.map((cell) => ({
      sourcePartIndex: cell.sourcePartIndex,
      noteId: cell.noteId,
    }))
    lyricCells = response.lyric_cells.map((cell) => ({
      sourcePartIndex: cell.sourcePartIndex,
      noteId: cell.noteId,
      verse: cell.verse,
    }))
  } else {
    const min = Math.min(anchorState.anchor.start, finalRange.start)
    const max = Math.max(anchorState.anchor.end, finalRange.end)
    const measureRange = { start: min, end: max }
    noteCells = noteCellsInMeasureRange(noteSpans, measureRange)
    lyricCells = lyricCellsInMeasureRange(lyricSpans, measureRange)
  }
  if (container) {
    applyPersistedNoteHighlights(container, noteCells)
    applyPersistedLyricHighlights(container, lyricCells)
  }
  return { noteCells, lyricCells }
}

/**
 * The bar-number-anchored sibling of `resolveMeasureSelection` above: instead
 * of selecting the exact measure-index range between the anchor and the
 * second click, expands *both* ends out to their own whole system first (via
 * `systemRangeContainingMeasure`), then selects the union of every measure in
 * between — every part, in every system from the anchor's system through the
 * resolved second click's system, regardless of what the second click landed
 * on (a note, a lyric syllable, another bar number, plain measure space —
 * `getMeasureAtPoint` already resolves any of those to a measure index, see
 * its own doc comment on scanning the full `elementsFromPoint` stack).
 *
 * Deliberately bypasses `resolve_selection_range`'s wasm path (unlike
 * `resolveMeasureSelection`): that path only knows about the exact measure
 * pair, not the system geometry doing the expansion here needs, and this
 * mode's whole point is to escalate past it — mirroring
 * `resolvePartLabelSystemSelection`'s own pure-TS, no-wasm resolution for the
 * same reason.
 */
export function resolveBarNumberSystemSelection(
  anchorState: BarNumberSystemAnchorState,
  { container, point, noteSpans, lyricSpans }: ResolveModeArgs,
): ResolvedSelection {
  const finalRange = point
    ? (getMeasureAtPoint(point.x, point.y) ?? anchorState.current)
    : anchorState.anchor

  const anchorSystem =
    (container &&
      systemRangeContainingMeasure(container, anchorState.anchor.start)) ||
    anchorState.anchor
  const finalSystem =
    (container && systemRangeContainingMeasure(container, finalRange.start)) ||
    finalRange

  const measureRange = {
    start: Math.min(anchorSystem.start, finalSystem.start),
    end: Math.max(anchorSystem.end, finalSystem.end),
  }
  const noteCells = noteCellsInMeasureRange(noteSpans, measureRange)
  const lyricCells = lyricCellsInMeasureRange(lyricSpans, measureRange)
  if (container) {
    applyPersistedNoteHighlights(container, noteCells)
    applyPersistedLyricHighlights(container, lyricCells)
  }
  return { noteCells, lyricCells }
}

export function resolveNoteSelection(
  anchorState: NoteAnchorState,
  { container, point, currentIdHint, noteSpans, lyricSpans }: ResolveModeArgs,
): ResolvedSelection {
  const current = point ?? anchorState.anchor
  const anchorCell: NoteCell = {
    sourcePartIndex: anchorState.anchorId.sourcePartIndex,
    noteId: anchorState.anchorId.noteId,
  }
  // Resolves every `Note ↔ Note` combination (same-part and cross-part)
  // and, when `current` lands on a lyric syllable, part label, or lyric
  // label instead, every one of `Note ↔ Lyric` (cross-row, either
  // ordering), `Note ↔ PartLabel`, and `Note ↔ LyricLabel` (see
  // `resolve_selection_range_response` in `selection_range.rs`) without
  // touching pixels at all, so it's immune to the scroll-invalidated-anchor
  // bug class. This covers every pair wasm can actually be asked to
  // resolve — it does NOT cover `current` missing every recognized click
  // target of any type (a bar-line/gutter or empty-space point), which
  // isn't any resolvable pair at all and has no ID to hand wasm. That case
  // keeps the pixel marquee (`applyNoteRangeSelection`'s fallback branch)
  // exactly as before this mode's wasm migration; collapsing straight to
  // the anchor here (as an earlier revision of this branch did) silently
  // regressed the cross-row case instead of falling back to it (see
  // `note-lyric-cross-range-select.feature`).
  const currentId =
    currentIdHint ?? anyClickableElementIdAtPoint(current.x, current.y)
  if (currentId === undefined) {
    let { noteCells, lyricCells } = container
      ? applyNoteRangeSelection(
          container,
          noteSpans,
          anchorCell,
          anchorState.anchor,
          current,
        )
      : { noteCells: [], lyricCells: [] }
    if (noteCells.length === 0) {
      noteCells = [anchorCell]
      lyricCells = []
      if (container) {
        applyPersistedNoteHighlights(container, noteCells)
        applyPersistedLyricHighlights(container, [])
      }
    }
    return { noteCells, lyricCells }
  }

  const response = resolve_selection_range(
    noteSpans,
    lyricSpans,
    anchorState.anchorId,
    currentId,
  )
  if (response.status !== 'ok') {
    // Should be unreachable — every type `current` can resolve to now has
    // an arm in `resolve_selection_range_response` paired with `Note`.
    // Logged rather than thrown so a real click-and-click gesture never
    // hard-fails on it; the empty-selection fallback below still collapses
    // to the anchor.
    console.error(
      'resolve_selection_range returned Err for a Note-anchored pair',
      anchorState.anchorId,
      currentId,
    )
  }

  let noteCells: NoteCell[] =
    response.status === 'ok'
      ? response.note_cells.map((cell) => ({
          sourcePartIndex: cell.sourcePartIndex,
          noteId: cell.noteId,
        }))
      : []
  let lyricCells: LyricCell[] =
    response.status === 'ok'
      ? response.lyric_cells.map((cell) => ({
          sourcePartIndex: cell.sourcePartIndex,
          noteId: cell.noteId,
          verse: cell.verse,
        }))
      : []
  if (container) {
    applyPersistedNoteHighlights(container, noteCells)
    applyPersistedLyricHighlights(container, lyricCells)
  }
  if (noteCells.length === 0) {
    // The resolved point missed every note's click target (e.g. it landed
    // back on the anchor note, or on a bar-line/gutter pixel) — fall back
    // to the cell the anchoring click resolved, so the gesture still
    // collapses to a single selection instead of an empty one.
    noteCells = [anchorCell]
    lyricCells = []
    if (container) {
      applyPersistedNoteHighlights(container, noteCells)
      applyPersistedLyricHighlights(container, [])
    }
  }
  return { noteCells, lyricCells }
}

export function resolveLyricSelection(
  anchorState: LyricAnchorState,
  { container, point, currentIdHint, noteSpans, lyricSpans }: ResolveModeArgs,
): ResolvedSelection {
  const current = point ?? anchorState.anchor
  const anchorCell: LyricCell = {
    sourcePartIndex: anchorState.anchorId.sourcePartIndex,
    noteId: anchorState.anchorId.noteId,
    verse: anchorState.anchorId.verse,
  }
  // Try wasm's ID-based range resolution first — resolves every
  // `Lyric ↔ Lyric` scope (same part-and-verse, same-part cross-verse, and
  // cross-part) and, when `current` lands on a note, part label, or lyric
  // label instead, every one of `Lyric ↔ Note` (cross-row),
  // `Lyric ↔ PartLabel`, and `Lyric ↔ LyricLabel` (see
  // `resolve_selection_range_response` in `selection_range.rs`) without
  // touching pixels at all. `Err` now only covers `current` missing every
  // recognized click target of any type — falls back to
  // `applyLyricRangeSelection`'s existing marquee path unchanged (the
  // same-mode mirror of 'note' mode's cross-row fallback above — see its
  // comment for why collapsing
  // straight through instead would silently regress the cross-row case).
  const currentId =
    currentIdHint ?? anyClickableElementIdAtPoint(current.x, current.y)
  const response = currentId
    ? resolve_selection_range(
        noteSpans,
        lyricSpans,
        anchorState.anchorId,
        currentId,
      )
    : undefined

  if (response?.status === 'ok') {
    const noteCells = response.note_cells.map((cell) => ({
      sourcePartIndex: cell.sourcePartIndex,
      noteId: cell.noteId,
    }))
    const lyricCells = response.lyric_cells.map((cell) => ({
      sourcePartIndex: cell.sourcePartIndex,
      noteId: cell.noteId,
      verse: cell.verse,
    }))
    if (container) {
      applyPersistedNoteHighlights(container, noteCells)
      applyPersistedLyricHighlights(container, lyricCells)
    }
    return { noteCells, lyricCells }
  }

  const { noteCells, lyricCells } = container
    ? applyLyricRangeSelection(
        container,
        lyricSpans,
        anchorCell,
        anchorState.anchor,
        current,
      )
    : { noteCells: [], lyricCells: [] }
  return { noteCells, lyricCells }
}
