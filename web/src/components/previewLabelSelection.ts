import type { LyricSpan, NoteSpan } from '../types'
import { clickableElementIdFromElement } from './clickableElementId'
import type { LyricCell, NoteCell } from './previewSelection'

/** The point-based counterpart of `clickableElementIdFromElement` used by
 * both label hit-tests below — mirrors `previewSelection.ts`'s own private
 * `getClickableElementIdAtPoint`. */
function getClickableElementIdAtPoint(x: number, y: number, tag: string) {
  const el = document.elementFromPoint(x, y)
  const group = el?.closest(`[data-tag="${tag}"]`)
  if (!group) return undefined
  return clickableElementIdFromElement(group)
}

/** One rendered part-label click target, keyed the same way as
 * `Tag::PartLabel`'s `data-part-index`/`data-measure-index-start`/
 * `data-measure-index-end` SVG attributes — see `getPartLabelAtPoint`. */
export interface PartLabelHit {
  sourcePartIndex: number
  measureIndexStart: number
  measureIndexEnd: number
}

/** The part-label click target under the given point, if any — reads the
 * invisible `PartLabelClickTarget` rect's enclosing `Tag::PartLabel` group
 * (see `renderer::new_renderer::render_part_label_click_target`). */
export function getPartLabelAtPoint(
  x: number,
  y: number,
): PartLabelHit | undefined {
  const id = getClickableElementIdAtPoint(x, y, 'part-label')
  if (id?.kind !== 'partLabel') return undefined
  return {
    sourcePartIndex: id.sourcePartIndex,
    measureIndexStart: id.measureIndexStart,
    measureIndexEnd: id.measureIndexEnd,
  }
}

/** Every note/rest cell belonging to the given part-label hits — each hit
 * selects its own part's notes across its own `measureIndexStart..=measureIndexEnd`
 * (the whole system the label sits in), mirroring `noteCellsInMeasureRange`. */
export function noteCellsForPartLabels(
  noteSpans: NoteSpan[],
  hits: PartLabelHit[],
): NoteCell[] {
  return hits.flatMap((hit) =>
    noteSpans
      .filter(
        (span) =>
          span.sourcePartIndex === hit.sourcePartIndex &&
          span.measureIndex >= hit.measureIndexStart &&
          span.measureIndex <= hit.measureIndexEnd,
      )
      .map((span) => ({
        sourcePartIndex: span.sourcePartIndex,
        noteId: span.noteId,
      })),
  )
}

/** Every lyric syllable cell belonging to the given part-label hits —
 * the lyric-side mirror of `noteCellsForPartLabels`, so a part-label
 * range-select selects the verse lyrics under its swept part rows alongside
 * their notes. */
export function lyricCellsForPartLabels(
  lyricSpans: LyricSpan[],
  hits: PartLabelHit[],
): LyricCell[] {
  return hits.flatMap((hit) =>
    lyricSpans
      .filter(
        (span) =>
          span.sourcePartIndex === hit.sourcePartIndex &&
          span.measureIndex >= hit.measureIndexStart &&
          span.measureIndex <= hit.measureIndexEnd,
      )
      .map((span) => ({
        sourcePartIndex: span.sourcePartIndex,
        noteId: span.noteId,
        verse: span.verse,
      })),
  )
}

/** One rendered lyric-label click target, keyed the same way as
 * `Tag::LyricLabel`'s `data-part-index`/`data-verse`/
 * `data-measure-index-start`/`data-measure-index-end` SVG attributes — see
 * `getLyricLabelAtPoint`. The lyric-side mirror of `PartLabelHit`. */
export interface LyricLabelHit {
  sourcePartIndex: number
  verse: number
  measureIndexStart: number
  measureIndexEnd: number
}

/** The lyric-label click target under the given point, if any — reads the
 * invisible `LyricLabelClickTarget` rect's enclosing `Tag::LyricLabel` group
 * (see `renderer::new_renderer::render_lyric_label_click_target`). The
 * lyric-side mirror of `getPartLabelAtPoint`. */
export function getLyricLabelAtPoint(
  x: number,
  y: number,
): LyricLabelHit | undefined {
  const id = getClickableElementIdAtPoint(x, y, 'lyric-label')
  if (id?.kind !== 'lyricLabel') return undefined
  return {
    sourcePartIndex: id.sourcePartIndex,
    verse: id.verse,
    measureIndexStart: id.measureIndexStart,
    measureIndexEnd: id.measureIndexEnd,
  }
}

/** Every lyric syllable cell belonging to the given lyric-label hits — each
 * hit selects its own verse's syllables across its own
 * `measureIndexStart..=measureIndexEnd` (the whole system the label sits
 * in), the lyric-side mirror of `noteCellsForPartLabels`. */
export function lyricCellsForLyricLabels(
  lyricSpans: LyricSpan[],
  hits: LyricLabelHit[],
): LyricCell[] {
  return hits.flatMap((hit) =>
    lyricSpans
      .filter(
        (span) =>
          span.sourcePartIndex === hit.sourcePartIndex &&
          span.verse === hit.verse &&
          span.measureIndex >= hit.measureIndexStart &&
          span.measureIndex <= hit.measureIndexEnd,
      )
      .map((span) => ({
        sourcePartIndex: span.sourcePartIndex,
        noteId: span.noteId,
        verse: span.verse,
      })),
  )
}
