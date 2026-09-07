import type { LyricSpan, NoteSpan } from '../types'
import { clickableElementIdFromElement } from './clickableElementId'

/** One rendered note/rest, keyed the same way as `Tag::Note`'s
 * `data-part-index`/`data-note-id` SVG attributes. */
export interface NoteCell {
  sourcePartIndex: number
  noteId: number
}

/**
 * Generic hit-test behind `getNoteAtPoint`/`getLyricAtPoint`/
 * `getPartLabelAtPoint`/`getLyricLabelAtPoint`: reads the element under
 * `(x, y)`, walks up to its nearest `[data-tag="{tag}"]` ancestor group, and
 * resolves a `ClickableElementId` off it via `clickableElementIdFromElement`
 * — the point-based counterpart of that function's own delegated-event use
 * (`mouseover`'s `event.target.closest(...)` in `usePreviewClickSelection.ts`).
 */
function getClickableElementIdAtPoint(x: number, y: number, tag: string) {
  const el = document.elementFromPoint(x, y)
  const group = el?.closest(`[data-tag="${tag}"]`)
  if (!group) return undefined
  return clickableElementIdFromElement(group)
}

export function getSectionLabelAtPoint(
  x: number,
  y: number,
): string | undefined {
  const el = document.elementFromPoint(x, y)
  if (!el) return undefined
  const group = el.closest('[data-tag="section-label"]')
  if (!group) return undefined
  return (group as HTMLElement).dataset.sectionLabel
}

export interface MeasureRange {
  start: number
  end: number
}

/** Adapts a `ClickableElementId`'s `'measure'` variant to this module's own
 * `MeasureRange` shape, or `undefined` if `id` resolved to some other kind —
 * shared by every measure-flavored hit-test below. */
function measureRangeFromId(
  id: ReturnType<typeof clickableElementIdFromElement>,
): MeasureRange | undefined {
  if (id?.kind !== 'measure') return undefined
  return { start: id.measureIndexStart, end: id.measureIndexEnd }
}

/**
 * Resolves a click point that's over a bar line's own click target
 * (`[data-tag="bar-line"]`, see `Tag::BarLine`/
 * `AbsoluteContent::BarLineClickTarget`) to a measure range, purely from the
 * server-computed `data-measure-index-next`/`data-measure-index-prev`
 * identity carried on that group (see `clickableElementIdFromElement`'s
 * `'bar-line'` case) — no pixel geometry involved beyond the initial
 * `elementFromPoint`.
 *
 * Returns `undefined` when `x`/`y` isn't over a bar-line click target at
 * all, so callers can fall through to the generic point-based lookup.
 *
 * Exported so `previewClickHandler.ts` can check for a bar-line hit
 * *before* its Cmd/Ctrl gate: grabbing the divider itself is an unambiguous
 * request to select measures, unlike a click on a note/lyric/gutter pixel
 * (which is ambiguous enough to need the modifier) — see that file's
 * unconditional bar-line check.
 */
export function getBarLineMeasureAtPoint(
  x: number,
  y: number,
): MeasureRange | undefined {
  return measureRangeFromId(getClickableElementIdAtPoint(x, y, 'bar-line'))
}

/**
 * Resolves a click point that's over a measure's own bar number (drawn
 * in the directive row above the musical rows — `[data-tag="bar-number"]`,
 * see `BarNumberClickTarget`/`Tag::BarNumber`) to that measure's range.
 *
 * A bar number sits outside every note's own click target. Kept as its own
 * unconditional (no Cmd/Ctrl needed) check in `previewClickHandler.ts`,
 * mirroring `getBarLineMeasureAtPoint`'s bar-line-handle check: landing on
 * the bar number itself is an unambiguous request to select that measure,
 * checked ahead of the Cmd/Ctrl gate rather than left to fall through to the
 * gutter-miss measure fallback further down.
 */
export function getBarNumberMeasureAtPoint(
  x: number,
  y: number,
): MeasureRange | undefined {
  return measureRangeFromId(getClickableElementIdAtPoint(x, y, 'bar-number'))
}

/**
 * Elements a click/hover point can resolve a measure range from:
 * `[data-tag="measure"]` (a measure's own musical-row body) and
 * `[data-tag="bar-number"]` (that same measure's own bar number, drawn in
 * the directive row above — see `BarNumberClickTarget`/`Tag::BarNumber`,
 * kept as a separate, smaller tag rather than folded into `[data-tag="measure"]`
 * so several e2e tests' `[data-tag="measure"]` DOM-order/count assumptions
 * stay unaffected).
 */
const MEASURE_RANGE_SELECTOR = '[data-tag="measure"], [data-tag="bar-number"]'

/**
 * The `MeasureRange` of the whole *system* that `measureIndex` belongs to —
 * used to expand a 'bar-number-system' gesture's endpoints out to their
 * full system before selecting (see `resolveBarNumberSystemSelection`).
 *
 * Reads it off `[data-tag="part-label"]`'s own `measureIndexStart`/
 * `measureIndexEnd` dataset pair rather than any dedicated "which system is
 * this measure in" lookup — every part label in a given system shares that
 * pair (one `PartLabelClickTarget` per part *per system*, see
 * `previewLabelRangeHighlights.ts`'s own doc comments), so scanning for the
 * one whose range contains `measureIndex` reliably identifies its system.
 * Returns `undefined` if no part label's range covers it (e.g. no parts are
 * rendered at all) — callers fall back to the bare measure range instead.
 */
export function systemRangeContainingMeasure(
  container: HTMLElement,
  measureIndex: number,
): MeasureRange | undefined {
  for (const label of Array.from(
    container.querySelectorAll<HTMLElement>('[data-tag="part-label"]'),
  )) {
    const { measureIndexStart, measureIndexEnd } = label.dataset
    if (measureIndexStart === undefined || measureIndexEnd === undefined)
      continue
    const start = Number.parseInt(measureIndexStart, 10)
    const end = Number.parseInt(measureIndexEnd, 10)
    if (measureIndex >= start && measureIndex <= end) return { start, end }
  }
  return undefined
}

export function getMeasureAtPoint(
  x: number,
  y: number,
): MeasureRange | undefined {
  const barLineRange = getBarLineMeasureAtPoint(x, y)
  if (barLineRange) return barLineRange

  // A measure's click-target rect is a sibling of the note/lyric click
  // targets drawn over the same region (see `render_measure_click_target`/
  // `render_note_click_target`), not their ancestor — so a point over a
  // note or lyric returns *that* element from `elementFromPoint`, and
  // `closest` never reaches the measure group underneath it. Scanning the
  // full `elementsFromPoint` stack instead finds the measure regardless of
  // what's painted on top of it at this point.
  for (const el of document.elementsFromPoint(x, y)) {
    const group = el.closest<HTMLElement>(MEASURE_RANGE_SELECTOR)
    if (group) {
      const range = measureRangeFromId(clickableElementIdFromElement(group))
      if (range) return range
    }
  }
  return undefined
}

/**
 * Every note/rest cell belonging to the given measure range, resolved from
 * `noteSpans`' `(source_part_index, note_id) → measure_index` mapping
 * (the same source-of-truth `groupSelectedNotesIntoContiguousRuns` groups
 * by) rather than a pixel-geometry intersection test. A geometric approach
 * (unioning the range's measure rects' bounding boxes and marquee-testing
 * note rects against it) previously seemed safe since a measure's
 * click-target rect and its boundary notes' rects are built off identical
 * column math and so should only ever touch, never overlap — but two
 * different SVG elements reporting bit-identical `getBoundingClientRect`
 * values for logically-identical coordinates isn't guaranteed (sub-pixel
 * rounding down independent transform chains), so a boundary note could
 * intermittently be pulled into the wrong neighboring measure's selection.
 * Resolving by index instead sidesteps that class of bug entirely.
 */
export function noteCellsInMeasureRange(
  noteSpans: NoteSpan[],
  range: MeasureRange,
): NoteCell[] {
  return noteSpans
    .filter(
      (span) =>
        span.measureIndex >= range.start && span.measureIndex <= range.end,
    )
    .map((span) => ({
      sourcePartIndex: span.sourcePartIndex,
      noteId: span.noteId,
    }))
}

/** The note/rest under the given point, if any — reads the invisible
 * `NoteClickTarget` rect's enclosing `Tag::Note` group (see
 * `renderer::new_renderer::render_note_click_target`), which sits on top of
 * the `pointer-events: none` playback cursor rect for the same note. */
export function getNoteAtPoint(x: number, y: number): NoteCell | undefined {
  const id = getClickableElementIdAtPoint(x, y, 'note')
  if (id?.kind !== 'note') return undefined
  return { sourcePartIndex: id.sourcePartIndex, noteId: id.noteId }
}

/** One rendered lyric syllable, keyed the same way as `Tag::Lyric`'s
 * `data-part-index`/`data-note-id`/`data-verse` SVG attributes. Structurally
 * identical to `NoteCell` but kept as its own type — a lyric cell and a note
 * cell sharing the same underlying note number are not interchangeable,
 * they're just keyed by the same note for convenience (see
 * `lyric_spans::LyricCell`). */
export interface LyricCell {
  sourcePartIndex: number
  noteId: number
  verse: number
}

/** The lyric syllable under the given point, if any — reads the invisible
 * `LyricClickTarget` rect's enclosing `Tag::Lyric` group (see
 * `renderer::new_renderer::render_lyric_click_target`), which paints on top
 * of the wider `NoteClickTarget` rect that geometrically covers the same
 * lyric row, so a click that lands on the syllable's own rect always
 * resolves here rather than to `getNoteAtPoint`. */
export function getLyricAtPoint(x: number, y: number): LyricCell | undefined {
  const id = getClickableElementIdAtPoint(x, y, 'lyric')
  if (id?.kind !== 'lyric') return undefined
  return {
    sourcePartIndex: id.sourcePartIndex,
    noteId: id.noteId,
    verse: id.verse,
  }
}

/** Every lyric syllable cell belonging to the given measure range, resolved
 * from `lyricSpans`' `(source_part_index, note_id, verse) → measure_index`
 * mapping — the lyric-side mirror of `noteCellsInMeasureRange`, so a measure
 * click can select the verse lyrics under it alongside its notes. */
export function lyricCellsInMeasureRange(
  lyricSpans: LyricSpan[],
  range: MeasureRange,
): LyricCell[] {
  return lyricSpans
    .filter(
      (span) =>
        span.measureIndex >= range.start && span.measureIndex <= range.end,
    )
    .map((span) => ({
      sourcePartIndex: span.sourcePartIndex,
      noteId: span.noteId,
      verse: span.verse,
    }))
}
