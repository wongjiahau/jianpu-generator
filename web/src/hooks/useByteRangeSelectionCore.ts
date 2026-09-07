import type { RefObject } from 'react'
import { useCallback, useRef, useState } from 'react'
import type { EditorHandle } from '../types'

/** A single Monaco multicursor range, in byte offsets into the source. */
export interface ByteRange {
  start: number
  end: number
}

export interface ByteRangeSelectionCore<Cell, Run> {
  /** The raw cells behind `runs`, kept around so the SVG preview can
   * re-apply the same highlight after any DOM change (e.g. a re-render
   * triggered by the Monaco selection a range-select just pushed). */
  selectedCells: Cell[]
  runs: Run[]
  handleRangeSelect: (selectedCells: Cell[]) => Promise<void>
  /** `ranges` is every disjoint Monaco selection (`getSelections()`), not
   * just the primary one — a multicursor selection (e.g. a clicked part
   * label's per-measure ranges, or the "shift selection octave" toolbar
   * action's remapped ranges) must overlap-test against all of them, or
   * cells sitting in every range but the first would silently drop out of
   * the derived SVG highlight even though Monaco's own text selection still
   * covers them. */
  handleEditorSelectionChange: (ranges: ByteRange[]) => Promise<void>
  /** Commits `cells`/`runs` as this core's state without pushing a Monaco
   * selection of its own — for a caller that's about to push a *combined*
   * multicursor selection spanning more than just this core's own cells
   * (e.g. a measure click's note cells and lyric cells together, see
   * `useAppController`'s `handleMeasureRangeSelect`), so each core's own
   * `editorRef.current.setSelections` doesn't clobber the other's. Also arms
   * `suppressNextEditorSelectionSyncCountRef`, since the caller's own combined
   * push still fires this editor's cursor-change notification — and without
   * suppressing it, `handleEditorSelectionChange` would immediately re-derive
   * (and typically empty out) `selectedCells` from whatever byte range the
   * *other* core's cells happened to land on.
   *
   * @param pendingEditorEchoes How many of those cursor-change notifications
   *   to swallow before `handleEditorSelectionChange` resumes deriving from
   *   real selection changes again — defaults to 1 (a single `setSelections`
   *   call). A caller whose upcoming Monaco call fires more than one
   *   notification synchronously (e.g. `replaceContentWithSelections`, which
   *   does `executeEdits` *then* `setSelections` — two notifications, not
   *   one) must pass the real count, or the second, unsuppressed
   *   notification will still re-derive from stale `spans` right after the
   *   first is correctly swallowed. */
  applySelectionSilently: (
    cells: Cell[],
    runs: Run[],
    pendingEditorEchoes?: number,
  ) => void
  /** Resets `selectedCells`/`runs` back to empty with no Monaco round-trip —
   * for a caller that needs to drop a stale no-mounted-editor highlight this
   * core is still holding (e.g. a section/sequence jump in a Synced/shared
   * view, which replaces whatever a prior note/lyric tap left painted rather
   * than extending it). A no-op once an editor is mounted, since that path
   * never leaves a highlight this core owns lying around uncleared. */
  clearSelection: () => void
}

/**
 * The shared low-level core behind `useNoteSelection`/`useLyricSelection`:
 * turns a range-select of `Cell`s hit-tested off the SVG preview into a
 * Monaco multicursor selection (`handleRangeSelect`), and the reverse — sync
 * back from whatever's actually selected in Monaco, including a selection
 * made by typing/selecting in the editor directly (`handleEditorSelectionChange`).
 *
 * Deliberately *not* a merge of the two hooks — lyric selection must stay
 * independent of note highlighting (a lyric range-select never selects/highlights
 * notes and vice versa), so each hook still keeps its own call to this core
 * and its own state; only the state shape and the two handlers' logic are
 * shared.
 *
 * @param spans The full flat span list (`NoteSpan[]`/`LyricSpan[]`) selected
 *   cells are resolved against.
 * @param editorRef The mounted editor, if any — `null` in Synced/shared views.
 * @param groupSelectedCellsIntoRuns Groups selected cells into contiguous
 *   byte runs (the wasm `group_note_selection`/`group_lyric_selection` call).
 * @param cellFromSpan Builds a `Cell` from one span — used by
 *   `handleEditorSelectionChange`'s byte-overlap filter below.
 * @param runByteRange Extracts a `Run`'s byte range, for pushing into Monaco.
 * @param onNoMountedEditor Called with the freshly-grouped runs when no
 *   editor is mounted, instead of pushing a Monaco selection — e.g.
 *   `useNoteSelection`'s Synced/shared-view fallback to a plain measure-range
 *   selection. `useLyricSelection` passes none, preserving its no-op
 *   behavior in that case.
 */
export function useByteRangeSelectionCore<
  Cell,
  Span extends { start?: number; end?: number },
  Run,
>(
  spans: Span[],
  editorRef: RefObject<EditorHandle | null>,
  groupSelectedCellsIntoRuns: (cells: Cell[], spans: Span[]) => Promise<Run[]>,
  cellFromSpan: (span: Span) => Cell,
  runByteRange: (run: Run) => ByteRange,
  onNoMountedEditor?: (runs: Run[]) => void,
): ByteRangeSelectionCore<Cell, Run> {
  const [selectedCells, setSelectedCells] = useState<Cell[]>([])
  const [runs, setRuns] = useState<Run[]>([])
  // Set right before `handleRangeSelect` pushes a selection into Monaco, so
  // the next `handleEditorSelectionChange` call(s) — which fire synchronously
  // off that same Monaco call, echoing the selection back — can no-op
  // instead of re-deriving `selectedCells` from them. That re-derivation
  // drops any cell with no byte span (e.g. a rest, which never became part
  // of the pushed Monaco selection in the first place), which would
  // otherwise silently shrink the preview highlight right after every
  // range-select that touched one. A counter, not a boolean: some callers' Monaco call
  // fires more than one cursor-change notification synchronously (see
  // `applySelectionSilently`'s `pendingEditorEchoes` doc comment), and a
  // boolean would only swallow the first of those, leaving the rest to
  // re-derive from stale `spans` anyway.
  const suppressNextEditorSelectionSyncCountRef = useRef(0)

  const handleRangeSelect = useCallback(
    async (cells: Cell[]) => {
      const newRuns = await groupSelectedCellsIntoRuns(cells, spans)
      if (!editorRef.current) {
        // No mounted editor (e.g. a Synced/shared view): a caller supplying
        // `onNoMountedEditor` handles this case entirely itself (see
        // `useNoteSelection`'s measure-range fallback) and deliberately
        // leaves `selectedCells`/`runs` untouched rather than reflecting a
        // selection with no Monaco round-trip and no note-selection
        // playback UI to drive. A caller with no fallback (`useLyricSelection`)
        // still records the selection, same as the editor-mounted path below
        // minus the actual Monaco push.
        if (onNoMountedEditor) {
          onNoMountedEditor(newRuns)
          return
        }
        setSelectedCells(cells)
        setRuns(newRuns)
        return
      }
      setSelectedCells(cells)
      setRuns(newRuns)
      if (newRuns.length === 0) return
      suppressNextEditorSelectionSyncCountRef.current = 1
      editorRef.current.setSelections(newRuns.map(runByteRange))
    },
    [
      spans,
      editorRef,
      groupSelectedCellsIntoRuns,
      onNoMountedEditor,
      runByteRange,
    ],
  )

  const handleEditorSelectionChange = useCallback(
    async (ranges: ByteRange[]) => {
      if (suppressNextEditorSelectionSyncCountRef.current > 0) {
        suppressNextEditorSelectionSyncCountRef.current -= 1
        return
      }
      const nonEmptyRanges = ranges.filter((range) => range.start !== range.end)
      const cells: Cell[] =
        nonEmptyRanges.length === 0
          ? []
          : spans
              .filter(
                (span): span is typeof span & { start: number; end: number } =>
                  span.start !== undefined && span.end !== undefined,
              )
              .filter((span) =>
                nonEmptyRanges.some(
                  (range) => span.start < range.end && span.end > range.start,
                ),
              )
              .map(cellFromSpan)
      setSelectedCells(cells)
      setRuns(await groupSelectedCellsIntoRuns(cells, spans))
    },
    [spans, groupSelectedCellsIntoRuns, cellFromSpan],
  )

  const applySelectionSilently = useCallback(
    (cells: Cell[], newRuns: Run[], pendingEditorEchoes = 1) => {
      setSelectedCells(cells)
      setRuns(newRuns)
      suppressNextEditorSelectionSyncCountRef.current = pendingEditorEchoes
    },
    [],
  )

  const clearSelection = useCallback(() => {
    setSelectedCells([])
    setRuns([])
  }, [])

  return {
    selectedCells,
    runs,
    handleRangeSelect,
    handleEditorSelectionChange,
    applySelectionSilently,
    clearSelection,
  }
}
