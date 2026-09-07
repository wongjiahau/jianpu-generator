import type { RefObject } from 'react'
import type { LyricCell } from '../components/previewSelection'
import { group_lyric_selection } from '../jianpuWasm'
import type { EditorHandle, LyricSpan } from '../types'
import { ensureWasmInit } from '../wasmInit'
import { useByteRangeSelectionCore } from './useByteRangeSelectionCore'

/** One contiguous range-selected byte range within a single verse line of a
 * single part's single measure, as grouped by the wasm export
 * `group_lyric_selection` (`lyric_spans::group_selected_lyrics_into_contiguous_runs`
 * in Rust). */
export interface LyricSelectionRun {
  sourcePartIndex: number
  measureIndex: number
  startByte: number
  endByte: number
}

/** Calls the wasm `group_lyric_selection` export directly on the main
 * thread (bypassing the debounced render worker) — this is pure grouping
 * over an already-fetched flat `lyric_spans` array, so it doesn't need to
 * re-parse `source` and stays responsive on every selection-change tick. */
/** Exported for `useAppController`'s `handleMeasureRangeSelect`, which groups
 * a measure click's lyric cells itself so it can push them into Monaco in
 * the same combined selection as the measure's note cells (see
 * `useByteRangeSelectionCore`'s `applySelectionSilently`). */
export async function groupSelectedLyricsIntoContiguousRuns(
  selectedCells: LyricCell[],
  lyricSpans: LyricSpan[],
): Promise<LyricSelectionRun[]> {
  await ensureWasmInit()
  const response = group_lyric_selection(lyricSpans, selectedCells)
  return response.status === 'ok'
    ? response.runs.map((r) => ({
        sourcePartIndex: r.sourcePartIndex,
        measureIndex: r.measureIndex,
        startByte: r.startByte,
        endByte: r.endByte,
      }))
    : []
}

function cellFromLyricSpan(span: LyricSpan): LyricCell {
  return {
    sourcePartIndex: span.sourcePartIndex,
    noteId: span.noteId,
    verse: span.verse,
  }
}

/** Exported alongside `groupSelectedLyricsIntoContiguousRuns` for
 * `useAppController`'s `handleMeasureRangeSelect`. */
export function lyricRunByteRange(run: LyricSelectionRun) {
  return { start: run.startByte, end: run.endByte }
}

/**
 * Turns a click-and-click range-select over lyric syllables (a set of
 * `(source_part_index, note_id, verse)` cells hit-tested off the SVG, see
 * `Preview.tsx`) into a Monaco multicursor selection over the source text —
 * one disjoint range per `(part, verse, measure)` the selection touched.
 *
 * Deliberately independent of `useNoteSelection`: a lyric selection never
 * drives note highlighting and vice versa, so this hook keeps its own call to
 * `useByteRangeSelectionCore` (and thus its own state) rather than threading
 * lyric cells through `useNoteSelection`'s `selectedNoteCells`/`runs`.
 */
export function useLyricSelection(
  lyricSpans: LyricSpan[],
  editorRef: RefObject<EditorHandle | null>,
) {
  const {
    selectedCells: lastSelectedCells,
    handleRangeSelect: handleLyricRangeSelect,
    handleEditorSelectionChange,
    applySelectionSilently: applyLyricSelectionSilently,
    clearSelection: clearLyricSelection,
  } = useByteRangeSelectionCore<LyricCell, LyricSpan, LyricSelectionRun>(
    lyricSpans,
    editorRef,
    groupSelectedLyricsIntoContiguousRuns,
    cellFromLyricSpan,
    lyricRunByteRange,
  )

  return {
    handleLyricRangeSelect,
    handleEditorSelectionChange,
    selectedLyricCells: lastSelectedCells,
    applyLyricSelectionSilently,
    clearLyricSelection,
  }
}
