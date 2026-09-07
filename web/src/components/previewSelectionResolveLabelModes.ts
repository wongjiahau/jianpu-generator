import { resolve_selection_range } from '../jianpuWasm'
import { anyClickableElementIdAtPoint } from './previewClickableElementIdBuilders'
import {
  applyPersistedLyricHighlights,
  applyPersistedNoteHighlights,
} from './previewRangeHighlights'
import type { PreviewAnchorState } from './previewAnchorState'
import {
  applyPersistedLyricLabelHighlights,
  applyPersistedPartLabelHighlights,
  lyricLabelsInMarquee,
  partLabelsInMarquee,
  partLabelsInMarqueeAcrossSystems,
} from './previewLabelRangeHighlights'
import {
  lyricCellsForLyricLabels,
  lyricCellsForPartLabels,
  noteCellsForPartLabels,
  type PartLabelHit,
} from './previewLabelSelection'
import type { LyricCell, NoteCell } from './previewSelection'
import type { ResolveModeArgs } from './previewSelectionResolveModes'
import type { ResolvedSelection } from './previewSelectionResolver'

type PartLabelAnchorState = Extract<
  NonNullable<PreviewAnchorState>,
  { mode: 'part-label' }
>
type PartLabelSystemAnchorState = Extract<
  NonNullable<PreviewAnchorState>,
  { mode: 'part-label-system' }
>
type LyricLabelAnchorState = Extract<
  NonNullable<PreviewAnchorState>,
  { mode: 'lyric-label' }
>

export function resolvePartLabelSelection(
  anchorState: PartLabelAnchorState,
  { container, point, currentIdHint, noteSpans, lyricSpans }: ResolveModeArgs,
): ResolvedSelection {
  const current = point ?? anchorState.anchor
  // Try wasm's ID-based range resolution first — resolves `PartLabel ↔
  // PartLabel` for any system pairing and, when `current` lands on a note,
  // lyric, or lyric label instead, every one of `PartLabel ↔ Note`,
  // `PartLabel ↔ Lyric`, and `PartLabel ↔ LyricLabel` (see
  // `resolve_selection_range_response` in `selection_range.rs`) without
  // touching pixels at all, no Cmd/Ctrl modifier required. `Err` only
  // covers `current` missing every recognized click target of any type,
  // which falls back to `partLabelsInMarquee`'s existing marquee path
  // unchanged.
  const currentId = container
    ? (currentIdHint ?? anyClickableElementIdAtPoint(current.x, current.y))
    : undefined
  const response = currentId
    ? resolve_selection_range(
        noteSpans,
        lyricSpans,
        anchorState.anchorId,
        currentId,
      )
    : undefined

  let noteCells: NoteCell[]
  let lyricCells: LyricCell[]
  if (response?.status === 'ok') {
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
    const hits: PartLabelHit[] = container
      ? partLabelsInMarquee(
          container,
          anchorState.anchor,
          current,
          anchorState.anchorId,
        )
      : []
    noteCells = noteCellsForPartLabels(noteSpans, hits)
    // Only union in the lyric row underneath when the resolved point
    // actually swept past a different label — a plain click that
    // resolves back to the same single label it anchored on selects just
    // that part's notes, not its lyrics too (see the "does not also
    // select the lyric row" regression test in
    // `part-label-click-selects-notes.feature`).
    lyricCells =
      hits.length > 1 ? lyricCellsForPartLabels(lyricSpans, hits) : []
  }
  if (container) {
    applyPersistedNoteHighlights(container, noteCells)
    applyPersistedLyricHighlights(container, lyricCells)
    // Replaces the transient hover-driven fill with the persisted one
    // immediately, rather than waiting for `selectedNoteCells` to
    // round-trip back down as a prop — otherwise every swept label's fill
    // would flash off for a frame between commit and that round-trip
    // landing.
    applyPersistedPartLabelHighlights(container, noteSpans, noteCells)
  }
  return { noteCells, lyricCells }
}

export function resolvePartLabelSystemSelection(
  anchorState: PartLabelSystemAnchorState,
  { container, point, noteSpans, lyricSpans }: ResolveModeArgs,
): ResolvedSelection {
  const current = point ?? anchorState.anchor
  // A separate, coarser Cmd/Ctrl-gated tool, kept distinct from the plain
  // range-select above even though that range-select is now itself
  // system-agnostic: this mode unions *every part* across every system the
  // gesture touches, where the plain range-select only ranges over the two
  // endpoints' own `sourcePartIndex`es (see
  // `resolve_selection_range_response`'s `PartLabel ↔ PartLabel` arm) — e.g.
  // a plain range-select from one part's label to that same part's label
  // two systems later selects only that
  // part, not every part in between, which this mode still does. See
  // `PLAN-clickable-element-id-selection.md`'s Status section.
  const hits = container
    ? partLabelsInMarqueeAcrossSystems(container, anchorState.anchor, current)
    : []
  const noteCells = noteCellsForPartLabels(noteSpans, hits)
  const lyricCells = lyricCellsForPartLabels(lyricSpans, hits)
  if (container) {
    applyPersistedNoteHighlights(container, noteCells)
    applyPersistedLyricHighlights(container, lyricCells)
    // Mirrors 'part-label' mode's own resolution above — see its comment.
    applyPersistedPartLabelHighlights(container, noteSpans, noteCells)
  }
  return { noteCells, lyricCells }
}

export function resolveLyricLabelSelection(
  anchorState: LyricLabelAnchorState,
  { container, point, currentIdHint, noteSpans, lyricSpans }: ResolveModeArgs,
): ResolvedSelection {
  const current = point ?? anchorState.anchor
  // Try wasm's ID-based range resolution first — resolves the same-verse
  // `LyricLabel ↔ LyricLabel` combination (any system pairing) and, when
  // `current` lands on a note, lyric, or part label instead, every one of
  // `LyricLabel ↔ Note`, `LyricLabel ↔ Lyric`, and `LyricLabel ↔ PartLabel`
  // (see `resolve_selection_range_response` in `selection_range.rs`) without
  // touching pixels at all. `Err` covers a different-verse `LyricLabel ↔
  // LyricLabel` pair (still out of scope — see that arm's own doc comment)
  // and `current` missing every recognized click target of any type — both
  // fall back to `lyricLabelsInMarquee`'s existing marquee path unchanged.
  const currentId = container
    ? (currentIdHint ?? anyClickableElementIdAtPoint(current.x, current.y))
    : undefined
  const response = currentId
    ? resolve_selection_range(
        noteSpans,
        lyricSpans,
        anchorState.anchorId,
        currentId,
      )
    : undefined

  let noteCells: NoteCell[]
  let lyricCells: LyricCell[]
  if (response?.status === 'ok') {
    // Unlike the plain `LyricLabel ↔ LyricLabel` pair (never any notes — a
    // lyric-only gesture never reaches into the note row), the label-mixed
    // `Note ↔ LyricLabel`/`PartLabel ↔ LyricLabel` pairs above always
    // populate `note_cells` too (see each arm's own doc comment in
    // `selection_range.rs`), so this mode can no longer hardcode an empty
    // `noteCells` the way it did before those arms existed.
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
    const hits = container
      ? lyricLabelsInMarquee(
          container,
          anchorState.anchor,
          current,
          anchorState.anchorId,
        )
      : []
    noteCells = []
    lyricCells = lyricCellsForLyricLabels(lyricSpans, hits)
  }
  if (container) {
    applyPersistedNoteHighlights(container, noteCells)
    applyPersistedLyricHighlights(container, lyricCells)
    // Mirrors 'part-label' mode's own resolution above — see its comment.
    applyPersistedLyricLabelHighlights(container, lyricSpans, lyricCells)
  }
  return { noteCells, lyricCells }
}
