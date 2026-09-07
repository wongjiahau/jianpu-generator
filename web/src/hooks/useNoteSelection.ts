import type { RefObject } from 'react'
import { useMemo } from 'react'
import { group_note_selection } from '../jianpuWasm'
import type { EditorHandle, NoteSpan, PartInfo } from '../types'
import type { NoteCell, NoteSelectionRun } from '../utils/noteSpanSelection'
import { ensureWasmInit } from '../wasmInit'
import { useByteRangeSelectionCore } from './useByteRangeSelectionCore'

/** Calls the wasm `group_note_selection` export directly on the main
 * thread (bypassing the debounced render worker) — this is pure grouping
 * over an already-fetched flat `note_spans` array, so it doesn't need to
 * re-parse `source` and stays responsive on every selection-change tick. */
/** Exported for `useAppController`'s `handleMeasureRangeSelect`, which groups
 * a measure click's note cells itself so it can push them into Monaco in the
 * same combined selection as the measure's lyric cells (see
 * `useByteRangeSelectionCore`'s `applySelectionSilently`). */
export async function groupSelectedNotesIntoContiguousRuns(
  selectedCells: NoteCell[],
  noteSpans: NoteSpan[],
): Promise<NoteSelectionRun[]> {
  await ensureWasmInit()
  const response = group_note_selection(noteSpans, selectedCells)
  return response.status === 'ok'
    ? response.runs.map((r) => ({
        sourcePartIndex: r.sourcePartIndex,
        measureIndex: r.measureIndex,
        startByte: r.startByte,
        endByte: r.endByte,
      }))
    : []
}

function cellFromNoteSpan(span: NoteSpan): NoteCell {
  return { sourcePartIndex: span.sourcePartIndex, noteId: span.noteId }
}

/** Exported alongside `groupSelectedNotesIntoContiguousRuns` for
 * `useAppController`'s `handleMeasureRangeSelect`. */
export function noteRunByteRange(run: NoteSelectionRun) {
  return { start: run.startByte, end: run.endByte }
}

export interface SelectedNoteRangePlaybackInfo {
  minMeasureIndex: number
  maxMeasureIndex: number
  selectedPartNames: string[]
}

/**
 * Turns a MuseScore-style note range-select (a set of `(source_part_index,
 * note_id)` cells hit-tested off the SVG, see `Preview.tsx`) into a Monaco
 * multicursor selection over the source text — one disjoint range per
 * `(part, measure)` the selection touched — and derives the info a "play
 * selection" action needs (see `useMeasureAudioPlayback.playNoteSelection`).
 */
export function useNoteSelection(
  noteSpans: NoteSpan[],
  parts: PartInfo[],
  /** The same `enabledTracks` filter threaded through the `listNoteSpans`
   * worker message (see `useJianpuWorkerRenderRequests.ts`) — needed to
   * resolve `sourcePartIndex` correctly, since `noteSpans` is fetched with
   * hidden parts filtered/compacted out while `parts` (from `list_parts`)
   * always stays the full, unfiltered list. `undefined` means every part is
   * enabled. */
  enabledTracks: string[] | undefined,
  editorRef: RefObject<EditorHandle | null>,
) {
  // Synced/shared views never mount an Editor, so there's no Monaco
  // selection to round-trip through `handleEditorSelectionChange` — but a
  // plain note tap still has its own precise blue highlight (painted
  // directly on the SVG by `resolveNoteSelection`, independent of any
  // editor), so no fallback is needed here: `useByteRangeSelectionCore`'s
  // default no-mounted-editor behavior (just recording `selectedCells`/
  // `runs`) is exactly right. This used to report the tap as a caret-only
  // `notifySelection` covering the whole enclosing measure — appropriate
  // back when this fallback was the *only* visual feedback (see
  // `useSectionNavigation`'s `selectSectionRange`), but left over after
  // per-note highlighting made it redundant, it painted a spurious
  // whole-measure amber background under every single-note tap in Synced/
  // shared view (see the mobile bug report this comment accompanies).
  // `useMeasureRangeSelection`'s own no-mounted-editor branch still does
  // this deliberately for an actual measure/bar-line selection.

  const {
    selectedCells: lastSelectedCells,
    runs: lastRuns,
    handleRangeSelect: handleNoteRangeSelect,
    handleEditorSelectionChange,
    applySelectionSilently: applyNoteSelectionSilently,
    clearSelection: clearNoteSelection,
  } = useByteRangeSelectionCore<NoteCell, NoteSpan, NoteSelectionRun>(
    noteSpans,
    editorRef,
    groupSelectedNotesIntoContiguousRuns,
    cellFromNoteSpan,
    noteRunByteRange,
  )

  const selectedNoteRangePlaybackInfo =
    useMemo<SelectedNoteRangePlaybackInfo | null>(() => {
      if (lastRuns.length === 0) return null
      const measureIndices = lastRuns.map((run) => run.measureIndex)
      const partIndices = new Set(lastRuns.map((run) => run.sourcePartIndex))
      // `sourcePartIndex` comes from `noteSpans`, fetched via the
      // `listNoteSpans` worker message *with* the current `enabledTracks` —
      // hidden parts are filtered out of the compiled score before indices
      // are assigned, so `sourcePartIndex` is a compacted, visible-parts-only
      // index (see `list_note_spans_from_source`'s doc comment in
      // `note_spans.rs`). `parts` (from `list_parts`, sent with no
      // `enabledTracks`) is always the full, unfiltered declaration-order
      // list, so it must be filtered the same way before indexing.
      const visibleParts = enabledTracks
        ? parts.filter((part) => enabledTracks.includes(part.abbreviation))
        : parts
      const selectedPartNames = Array.from(partIndices)
        .map((partIndex) => visibleParts[partIndex]?.abbreviation)
        .filter((abbreviation): abbreviation is string => abbreviation != null)
      return {
        minMeasureIndex: Math.min(...measureIndices),
        maxMeasureIndex: Math.max(...measureIndices),
        selectedPartNames,
      }
    }, [lastRuns, parts, enabledTracks])

  return {
    handleNoteRangeSelect,
    handleEditorSelectionChange,
    selectedNoteRangePlaybackInfo,
    selectedNoteCells: lastSelectedCells,
    // Exposed alongside `selectedNoteCells` for `handleShiftSelectionOctave`
    // (see `useAppSelectionAndNavigation.ts`), which needs both to re-apply
    // the same selection silently after an octave shift.
    selectedNoteRuns: lastRuns,
    applyNoteSelectionSilently,
    clearNoteSelection,
  }
}
