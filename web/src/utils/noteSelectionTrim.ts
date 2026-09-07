import type { NoteTimingOut } from '../jianpuWasm'
import type { NoteCell } from './noteSpanSelection'

/** Elapsed-seconds window, relative to the start of a generated audio clip. */
export interface TrimWindow {
  start: number
  end: number
  /** Elapsed-seconds start of the next unselected note after `end`, if any
   * — passed to Rust as a hard cap so the release tail it adds past `end`
   * (see `crate::wav::TrimWindow::next_note_start_s`) can't bleed into that
   * note and be heard as one extra note beyond the selection. */
  nextNoteStart?: number
}

/**
 * Narrows a measure-range clip's note timings down to the elapsed-seconds
 * window that exactly covers a range-selected set of notes, so "play
 * selection" (see `useMeasureAudioPlayback.playNoteSelection`) can seek/stop
 * at the real note boundaries instead of playing the full boundary measures
 * the selection touches.
 *
 * Returns `null` when none of `cells` has a matching timing (nothing to
 * trim to — falls back to playing the clip in full) or the resulting window
 * is empty/inverted.
 */
export function computeNoteSelectionTrimWindow(
  cells: NoteCell[],
  noteTimings: NoteTimingOut[],
): TrimWindow | null {
  const keys = new Set(cells.map((c) => `${c.sourcePartIndex}:${c.noteId}`))
  const matched = noteTimings.filter((t) =>
    keys.has(`${t.source_part_index}:${t.note_id}`),
  )
  if (matched.length === 0) return null
  const start = Math.min(...matched.map((t) => t.start_s))
  const end = Math.max(...matched.map((t) => t.end_s))
  if (end <= start) return null
  // The earliest unselected note that starts at or after `end` — passed to
  // Rust as a hard cap on the release tail (see `TrimWindow.nextNoteStart`)
  // so a tightly-packed next note doesn't get partially/fully played,
  // which would sound like one extra note beyond the selection.
  const nextNoteStarts = noteTimings
    .filter((t) => !keys.has(`${t.source_part_index}:${t.note_id}`))
    .map((t) => t.start_s)
    .filter((s) => s >= end)
  const nextNoteStart =
    nextNoteStarts.length > 0 ? Math.min(...nextNoteStarts) : undefined
  return { start, end, nextNoteStart }
}
