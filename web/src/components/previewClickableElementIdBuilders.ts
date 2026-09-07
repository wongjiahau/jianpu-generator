import type { ClickableElementId } from './clickableElementId'
import type { LyricLabelHit, PartLabelHit } from './previewLabelSelection'
import {
  getLyricLabelAtPoint,
  getPartLabelAtPoint,
} from './previewLabelSelection'
import type { LyricCell, MeasureRange, NoteCell } from './previewSelection'
import { getLyricAtPoint, getNoteAtPoint } from './previewSelection'

/**
 * Builds the `ClickableElementId` (`'measure'` variant) `resolve_selection_range`
 * expects from a `MeasureRange`. Exported alongside its four siblings below
 * so `previewClickHandler.ts` can build a `PreviewAnchorState`'s `anchorId` at
 * anchor time, not just here at resolve time.
 */
export function measureClickableElementId(
  range: MeasureRange,
): Extract<ClickableElementId, { kind: 'measure' }> {
  return {
    kind: 'measure',
    measureIndexStart: range.start,
    measureIndexEnd: range.end,
  }
}

/** The 'note'-mode analog of `measureClickableElementId` — builds the
 * `ClickableElementId` (`'note'` variant) from a `NoteCell`. */
export function noteClickableElementId(
  cell: NoteCell,
): Extract<ClickableElementId, { kind: 'note' }> {
  return {
    kind: 'note',
    sourcePartIndex: cell.sourcePartIndex,
    noteId: cell.noteId,
  }
}

/** The 'lyric'-mode analog of `noteClickableElementId` — builds the
 * `ClickableElementId` (`'lyric'` variant) from a `LyricCell`. */
export function lyricClickableElementId(
  cell: LyricCell,
): Extract<ClickableElementId, { kind: 'lyric' }> {
  return {
    kind: 'lyric',
    sourcePartIndex: cell.sourcePartIndex,
    noteId: cell.noteId,
    verse: cell.verse,
  }
}

/** The 'part-label'-mode analog of `noteClickableElementId` — builds the
 * `ClickableElementId` (`'partLabel'` variant) from a `PartLabelHit`. */
export function partLabelClickableElementId(
  hit: PartLabelHit,
): Extract<ClickableElementId, { kind: 'partLabel' }> {
  return {
    kind: 'partLabel',
    sourcePartIndex: hit.sourcePartIndex,
    measureIndexStart: hit.measureIndexStart,
    measureIndexEnd: hit.measureIndexEnd,
  }
}

/** The 'lyric-label'-mode analog of `partLabelClickableElementId` — builds
 * the `ClickableElementId` (`'lyricLabel'` variant) from a `LyricLabelHit`. */
export function lyricLabelClickableElementId(
  hit: LyricLabelHit,
): Extract<ClickableElementId, { kind: 'lyricLabel' }> {
  return {
    kind: 'lyricLabel',
    sourcePartIndex: hit.sourcePartIndex,
    verse: hit.verse,
    measureIndexStart: hit.measureIndexStart,
    measureIndexEnd: hit.measureIndexEnd,
  }
}

/**
 * The `ClickableElementId` for whatever clickable target sits at `(x, y)`,
 * trying every type `resolve_selection_range` can resolve a range against —
 * `Note`, `Lyric`, `PartLabel`, then `LyricLabel` — and returning `undefined`
 * only when the point misses every one of them (a bar-line/gutter/
 * empty-space point). Every one of `'note'`/`'lyric'`/`'part-label'`/
 * `'lyric-label'` mode's `current` resolution goes through this now, not
 * just a same-type hit-test: with every `(anchor-type, current-type)`
 * combination among these four types ID-resolved (see
 * `resolve_selection_range_response`'s label-mixed arms in
 * `selection_range.rs`, added per
 * `PLAN-clickable-element-id-selection.md`'s Status section), a second click
 * landing on *any* of them — not just the anchor's own type — reaches wasm
 * instead of falling straight to a mode's pixel-marquee fallback. The order
 * mirrors `handleAnchorClick`'s own hit-test priority (label checks before
 * the plain note/lyric checks), except `Note`/`Lyric` are tried first here
 * since they're both mode's own most common `current` target and cheaper to
 * rule out than walking the label checks first.
 */
export function anyClickableElementIdAtPoint(
  x: number,
  y: number,
): ClickableElementId | undefined {
  const noteCell = getNoteAtPoint(x, y)
  if (noteCell) return noteClickableElementId(noteCell)
  const lyricCell = getLyricAtPoint(x, y)
  if (lyricCell) return lyricClickableElementId(lyricCell)
  const partLabelHit = getPartLabelAtPoint(x, y)
  if (partLabelHit) return partLabelClickableElementId(partLabelHit)
  const lyricLabelHit = getLyricLabelAtPoint(x, y)
  if (lyricLabelHit) return lyricLabelClickableElementId(lyricLabelHit)
  return undefined
}
