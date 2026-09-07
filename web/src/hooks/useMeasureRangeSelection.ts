import type { RefObject } from 'react'
import { useCallback, useState } from 'react'
import type { LyricCell, NoteCell } from '../components/previewSelection'
import type { EditorHandle, LyricSpan, MeasureSpan, NoteSpan } from '../types'
import {
  groupSelectedLyricsIntoContiguousRuns,
  lyricRunByteRange,
} from './useLyricSelection'
import {
  groupSelectedNotesIntoContiguousRuns,
  noteRunByteRange,
} from './useNoteSelection'

export interface UseMeasureRangeSelectionResult {
  handleMeasureRangeSelect: (
    noteCells: NoteCell[],
    lyricCells: LyricCell[],
  ) => Promise<void>
  /** The note/lyric cells behind the most recent no-mounted-editor
   * measure/bar-line selection — `[]` once an editor is mounted, since that
   * path pushes into `applyNoteSelectionSilently`/`applyLyricSelectionSilently`
   * instead (see this hook's own doc comment). Callers merge these into
   * whatever `useNoteSelection`/`useLyricSelection` themselves are tracking
   * (see `useAppSelectionAndNavigation`) purely so `Preview.tsx`'s
   * persisted-highlight effect has *something* non-stale to re-paint from on
   * every render — deliberately kept out of `useNoteSelection`/
   * `useLyricSelection`'s own `selectedCells`/`runs`, which also drive the
   * play-measure button's "Selection" label and Monaco playback (see the
   * no-mounted-editor branch below for why routing through those would be
   * wrong here). */
  measureRangeNoteCells: NoteCell[]
  measureRangeLyricCells: LyricCell[]
  /** Resets `measureRangeNoteCells`/`measureRangeLyricCells` back to empty —
   * for a caller that needs to drop a stale no-mounted-editor measure/
   * bar-line highlight this hook is still holding (e.g. a section/sequence
   * jump in a Synced/shared view; see those hooks' own callers). A no-op once
   * an editor is mounted, since that path never populates these in the first
   * place. */
  clearMeasureRangeSelection: () => void
}

/**
 * Turns a measure/bar-line click or range-select (see `Preview.tsx`'s
 * `onMeasureRangeSelect`) into a single combined Monaco multicursor
 * selection covering both the note cells and the lyric cells under it.
 *
 * Deliberately *not* just calling `handleNoteRangeSelect` and
 * `handleLyricRangeSelect` back-to-back: each independently pushes its own
 * Monaco selection when an editor is mounted, so the second call's
 * `setSelections` clobbers the first's, and the resulting
 * `onDidChangeCursorSelection` echo would then re-derive (and typically
 * empty out) whichever core's `selectedCells` wasn't the last one pushed.
 * This groups both sides itself and pushes one combined selection instead,
 * silently committing each core's `selectedCells`/`runs` beforehand (via
 * `applyNoteSelectionSilently`/`applyLyricSelectionSilently`, see
 * `useByteRangeSelectionCore`'s `applySelectionSilently`) so neither depends
 * on that editor echo.
 */
export function useMeasureRangeSelection(
  editorRef: RefObject<EditorHandle | null>,
  noteSpans: NoteSpan[],
  lyricSpans: LyricSpan[],
  applyNoteSelectionSilently: (
    cells: NoteCell[],
    runs: Awaited<ReturnType<typeof groupSelectedNotesIntoContiguousRuns>>,
  ) => void,
  applyLyricSelectionSilently: (
    cells: LyricCell[],
    runs: Awaited<ReturnType<typeof groupSelectedLyricsIntoContiguousRuns>>,
  ) => void,
  measureSpans: MeasureSpan[],
  notifySelection: (
    startLine: number,
    endLine: number,
    isEmpty: boolean,
  ) => void,
): UseMeasureRangeSelectionResult {
  const [measureRangeNoteCells, setMeasureRangeNoteCells] = useState<
    NoteCell[]
  >([])
  const [measureRangeLyricCells, setMeasureRangeLyricCells] = useState<
    LyricCell[]
  >([])

  // biome-ignore lint/correctness/useExhaustiveDependencies: editorRef is a ref object with a stable identity across renders (standard React convention); listing editorRef.current/.setSelections would stale-capture the ref's value at callback-creation time instead of reading it live on each call.
  const handleMeasureRangeSelect = useCallback(
    async (noteCells: NoteCell[], lyricCells: LyricCell[]) => {
      if (!editorRef.current) {
        // No mounted editor (Synced/shared view) — deliberately doesn't route
        // through `handleNoteRangeSelect`/`handleLyricRangeSelect` here (each
        // would populate `selectedCells`, flipping `noteSelectionActive` on
        // and hijacking the play-measure button's "Measures N–M" label into
        // "Selection" — see `PlayMeasureButton`'s doc comment): the note/
        // lyric cells themselves already got their own precise blue/lyric
        // highlight painted directly on the SVG (`resolveMeasureSelection`),
        // and the amber whole-measure background is reserved for an actual
        // Monaco caret (see `useJianpuWorkerRenderRequests.ts`'s
        // `notifySelection` — only a caret-only report paints it, same
        // convention `useSectionNavigation.selectSectionRange` relies on).
        // There's no Monaco caret in this view at all, so `notifySelection`
        // below is always called with `isEmpty: false`: it still updates
        // `selectedMeasureRange` for the play-measure button's "Measures
        // N–M" label/badge and playback range, but never paints the amber
        // background itself (see the mobile bug report this comment
        // accompanies: a bar-line tap in a mobile Synced/shared viewer
        // shouldn't paint it either).
        //
        // Still records `noteCells`/`lyricCells` here (via
        // `measureRangeNoteCells`/`measureRangeLyricCells`, *not*
        // `applyNoteSelectionSilently`/`applyLyricSelectionSilently`) so
        // `Preview.tsx` has a live React value to re-paint the SVG highlight
        // from on every render — the direct DOM paint `resolveMeasureSelection`
        // already did is otherwise silently wiped the moment `notifySelection`
        // below causes any re-render (even one that changes nothing visible,
        // e.g. `highlightedDocuments` flipping to a new empty-array
        // reference), since that re-render re-applies the (until now
        // permanently stale/empty) persisted-highlight state instead.
        setMeasureRangeNoteCells(noteCells)
        setMeasureRangeLyricCells(lyricCells)
        const [noteRuns, lyricRuns] = await Promise.all([
          groupSelectedNotesIntoContiguousRuns(noteCells, noteSpans),
          groupSelectedLyricsIntoContiguousRuns(lyricCells, lyricSpans),
        ])
        const measureIndices = [
          ...noteRuns.map((run) => run.measureIndex),
          ...lyricRuns.map((run) => run.measureIndex),
        ]
        if (measureIndices.length === 0) return
        const startSpan = measureSpans[Math.min(...measureIndices)]
        const endSpan = measureSpans[Math.max(...measureIndices)]
        if (startSpan && endSpan) {
          notifySelection(startSpan.start_line, endSpan.end_line, false)
        }
        return
      }
      const [noteRuns, lyricRuns] = await Promise.all([
        groupSelectedNotesIntoContiguousRuns(noteCells, noteSpans),
        groupSelectedLyricsIntoContiguousRuns(lyricCells, lyricSpans),
      ])
      applyNoteSelectionSilently(noteCells, noteRuns)
      applyLyricSelectionSilently(lyricCells, lyricRuns)
      const ranges = [
        ...noteRuns.map(noteRunByteRange),
        ...lyricRuns.map(lyricRunByteRange),
      ]
      if (ranges.length === 0) return
      editorRef.current.setSelections(ranges)
    },
    [
      noteSpans,
      lyricSpans,
      measureSpans,
      notifySelection,
      applyNoteSelectionSilently,
      applyLyricSelectionSilently,
    ],
  )

  const clearMeasureRangeSelection = useCallback(() => {
    setMeasureRangeNoteCells([])
    setMeasureRangeLyricCells([])
  }, [])

  return {
    handleMeasureRangeSelect,
    measureRangeNoteCells,
    measureRangeLyricCells,
    clearMeasureRangeSelection,
  }
}
