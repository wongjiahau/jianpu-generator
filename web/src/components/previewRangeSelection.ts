import type { LyricSpan, NoteSpan } from '../types'
import type { AnchorPoint } from './previewRangeHighlights'
import {
  applyLyricRangeHighlights,
  applyNoteRangeHighlights,
  applyPersistedLyricHighlights,
  applyPersistedNoteHighlights,
} from './previewRangeHighlights'
import {
  getLyricAtPoint,
  getNoteAtPoint,
  type LyricCell,
  type NoteCell,
} from './previewSelection'

export interface RangeSelection {
  noteCells: NoteCell[]
  lyricCells: LyricCell[]
}

/**
 * Every note/rest cell in `sourcePartIndex` whose `noteId` falls in
 * [min(idA, idB), max(idA, idB)] inclusive — the 'note'-mode analog of
 * `noteCellsInMeasureRange` (see `previewSelection.ts`): resolves a
 * same-part note-to-note range by `noteId` order (score order) rather than a
 * pixel-geometry marquee, so it's immune to both a scrolled/stale anchor
 * point and a marquee too narrow to span a system boundary.
 */
function noteCellsInNoteIdRange(
  noteSpans: NoteSpan[],
  sourcePartIndex: number,
  noteIdA: number,
  noteIdB: number,
): NoteCell[] {
  const min = Math.min(noteIdA, noteIdB)
  const max = Math.max(noteIdA, noteIdB)
  return noteSpans
    .filter(
      (span) =>
        span.sourcePartIndex === sourcePartIndex &&
        span.noteId >= min &&
        span.noteId <= max,
    )
    .map((span) => ({
      sourcePartIndex: span.sourcePartIndex,
      noteId: span.noteId,
    }))
}

/** The lyric-side mirror of `noteCellsInNoteIdRange`, additionally scoped to
 * one `verse` — used to resolve a same-verse LYRIC range (see
 * `applyLyricRangeSelection`), since a lyric-mode range-select should stay
 * within the one verse row it anchored on, mirroring how a note-mode
 * range-select stays within one part. */
function lyricCellsInVerseNoteIdRange(
  lyricSpans: LyricSpan[],
  sourcePartIndex: number,
  verse: number,
  noteIdA: number,
  noteIdB: number,
): LyricCell[] {
  const min = Math.min(noteIdA, noteIdB)
  const max = Math.max(noteIdA, noteIdB)
  return lyricSpans
    .filter(
      (span) =>
        span.sourcePartIndex === sourcePartIndex &&
        span.verse === verse &&
        span.noteId >= min &&
        span.noteId <= max,
    )
    .map((span) => ({
      sourcePartIndex: span.sourcePartIndex,
      noteId: span.noteId,
      verse: span.verse,
    }))
}

/**
 * Resolves and applies 'note' mode's range highlight between `anchor` and
 * `current`. When `current` lands directly on a note in the same part as
 * `anchorCell`, resolves by `noteId` order (`noteCellsInNoteIdRange`)
 * instead of pixel geometry — immune to both the scroll-invalidated-anchor
 * bug and the system-boundary marquee bug (see this repo's
 * `note-range-select-crosses-page.feature` / `-crosses-system.feature`).
 * Deliberately resolves *only* note cells on that path, not lyrics too:
 * unlike the marquee (whose zero-area single-point test naturally excludes
 * the lyric row below unless the marquee actually sweeps into it), an index
 * range has no notion of "row", so unconditionally unioning in every verse's
 * lyrics under the covered notes would select lyrics on a plain same-row
 * note-to-note range-select — see `lyric-syllable-independent-selection.feature`'s
 * "no lyric syllable is range-selected" coverage. Every other case (different
 * part, or `current` misses every note's click target — a lyric syllable,
 * bar-line/gutter, empty space mid-hover) keeps the existing pixel marquee
 * (`applyNoteRangeHighlights`/`applyLyricRangeHighlights`, which still unions
 * in lyrics when the marquee's geometry actually reaches their row) — this
 * is what still lets the marquee sweep vertically across part rows at the same
 * beat within one system, and cross into the lyric row underneath and back
 * (see `note-lyric-cross-range-select.feature`,
 * `note-range-select-highlight.feature`, left covered by the same marquee
 * path as before). Applies DOM highlights as a side effect (matching
 * `apply*RangeHighlights`'s shape) and returns the resolved cells, which can
 * be empty — the empty-result → `noteCellAtAnchor` collapse-to-single-cell
 * fallback stays the caller's job (`previewSelectionResolver.ts`'s 'note'
 * branch).
 *
 * The marquee fallback path (cross-part note range-selects, note↔lyric
 * cross-range-selects) keeps its existing scroll-sensitivity — a known,
 * accepted gap, not fixed here (see this repo's plan for the scope of this
 * change).
 */
export function applyNoteRangeSelection(
  container: HTMLElement,
  noteSpans: NoteSpan[],
  anchorCell: NoteCell,
  anchor: AnchorPoint,
  current: AnchorPoint,
): RangeSelection {
  const currentCell = getNoteAtPoint(current.x, current.y)
  if (
    currentCell !== undefined &&
    currentCell.sourcePartIndex === anchorCell.sourcePartIndex
  ) {
    const noteCells = noteCellsInNoteIdRange(
      noteSpans,
      anchorCell.sourcePartIndex,
      anchorCell.noteId,
      currentCell.noteId,
    )
    applyPersistedNoteHighlights(container, noteCells)
    applyPersistedLyricHighlights(container, [])
    return { noteCells, lyricCells: [] }
  }

  const noteCells = applyNoteRangeHighlights(container, anchor, current)
  const lyricCells = applyLyricRangeHighlights(container, anchor, current)
  return { noteCells, lyricCells }
}

/** The lyric-side mirror of `applyNoteRangeSelection`: when `current` lands
 * directly on a syllable in the same part AND VERSE as `anchorCell`,
 * resolves by `noteId` order (`lyricCellsInVerseNoteIdRange`) instead of
 * pixel geometry. Deliberately resolves *only* lyric cells on that path, for
 * the same reason `applyNoteRangeSelection` resolves only notes on its own
 * index path — see that function's doc comment and
 * `lyric-syllable-independent-selection.feature`'s "no note is range-selected"
 * coverage. Every other case keeps the existing marquee unchanged. No
 * empty-result fallback needed here (unlike 'note' mode): 'lyric' mode's
 * anchor click is only ever reached via a direct `getLyricAtPoint` hit (see
 * `previewClickHandler.ts`), so `current === anchor`'s self-commit always
 * re-resolves to the same single-syllable cell through the index path
 * itself. */
export function applyLyricRangeSelection(
  container: HTMLElement,
  lyricSpans: LyricSpan[],
  anchorCell: LyricCell,
  anchor: AnchorPoint,
  current: AnchorPoint,
): RangeSelection {
  const currentCell = getLyricAtPoint(current.x, current.y)
  if (
    currentCell !== undefined &&
    currentCell.sourcePartIndex === anchorCell.sourcePartIndex &&
    currentCell.verse === anchorCell.verse
  ) {
    const lyricCells = lyricCellsInVerseNoteIdRange(
      lyricSpans,
      anchorCell.sourcePartIndex,
      anchorCell.verse,
      anchorCell.noteId,
      currentCell.noteId,
    )
    applyPersistedLyricHighlights(container, lyricCells)
    applyPersistedNoteHighlights(container, [])
    return { noteCells: [], lyricCells }
  }

  const lyricCells = applyLyricRangeHighlights(container, anchor, current)
  const noteCells = applyNoteRangeHighlights(container, anchor, current)
  return { noteCells, lyricCells }
}
