import type { RefObject } from 'react'
import { useCallback, useMemo } from 'react'
import type {
  EditorHandle,
  EditorSelection,
  LyricSpan,
  MeasureSpan,
  NoteSpan,
  PartInfo,
  SectionRange,
  SequenceEntry,
} from '../types'
import type { NoteCell } from '../utils/noteSpanSelection'
import { useLyricSelection } from './useLyricSelection'
import { useMeasureRangeSelection } from './useMeasureRangeSelection'
import { useNoteSelection } from './useNoteSelection'
import { useSectionNavigation } from './useSectionNavigation'
import { useSequenceNavigation } from './useSequenceNavigation'

/** Wires together section/sequence jump navigation and note/lyric/measure
 * range selection — the editor-selection half of `useAppController`. Split
 * out to keep `useAppController` under its line-count cap. */
export function useAppSelectionAndNavigation(
  sectionRanges: SectionRange[],
  editorRef: RefObject<EditorHandle | null>,
  notifySelection: (
    firstLine: number,
    lastLine: number,
    isEmpty: boolean,
    revealLine?: number,
    measureRanges?: { start: number; end: number }[],
  ) => void,
  sequenceEntries: SequenceEntry[],
  measureSpans: MeasureSpan[],
  selectedSequenceRangeRef: RefObject<{
    start: number
    end: number
    entryStartIndex: number
    entryEndIndex: number
  } | null>,
  noteSpans: NoteSpan[],
  parts: PartInfo[],
  enabledTracks: string[] | undefined,
  lyricSpans: LyricSpan[],
  playNoteSelection: (
    minMeasureIndex: number,
    maxMeasureIndex: number,
    selectedPartNames: string[],
    selectedCells: NoteCell[],
  ) => void,
  /** The "shift selection octave" worker call (see
   * `source_edit::shift_range_octave`) and the source-change setter it
   * resolves into — threaded through so `handleShiftSelectionOctave` below
   * can re-derive and restore the note selection afterward. */
  shiftRangeOctave: (
    ranges: EditorSelection[],
    delta: number,
  ) => Promise<{ source: string; ranges: EditorSelection[] }>,
  handleSourceChange: (value: string) => void,
) {
  const {
    handleNoteRangeSelect,
    handleEditorSelectionChange,
    selectedNoteRangePlaybackInfo,
    selectedNoteCells: noteSelectionCells,
    selectedNoteRuns: noteSelectionRuns,
    applyNoteSelectionSilently,
    clearNoteSelection,
  } = useNoteSelection(noteSpans, parts, enabledTracks, editorRef)

  const {
    handleLyricRangeSelect,
    handleEditorSelectionChange: handleLyricEditorSelectionChange,
    selectedLyricCells: lyricSelectionCells,
    applyLyricSelectionSilently,
    clearLyricSelection,
  } = useLyricSelection(lyricSpans, editorRef)

  const {
    handleMeasureRangeSelect,
    measureRangeNoteCells,
    measureRangeLyricCells,
    clearMeasureRangeSelection,
  } = useMeasureRangeSelection(
    editorRef,
    noteSpans,
    lyricSpans,
    applyNoteSelectionSilently,
    applyLyricSelectionSilently,
    measureSpans,
    notifySelection,
  )

  // Fed to `useSectionNavigation`/`useSequenceNavigation` below — a section
  // or sequence jump replaces whatever a prior no-mounted-editor (Synced/
  // shared view) note/lyric tap or measure/bar-line click left painted,
  // rather than layering on top of it (see those hooks' own
  // `clearNoMountedEditorHighlights` param doc comment).
  const clearNoMountedEditorHighlights = useCallback(() => {
    clearNoteSelection()
    clearLyricSelection()
    clearMeasureRangeSelection()
  }, [clearNoteSelection, clearLyricSelection, clearMeasureRangeSelection])

  const { setSelectedLineRange, handleSectionJump, sectionJumpToolbarProps } =
    useSectionNavigation(
      sectionRanges,
      editorRef,
      measureSpans,
      notifySelection,
      clearNoMountedEditorHighlights,
    )

  const { selectedSequenceRange, sequenceJumpToolbarProps } =
    useSequenceNavigation(
      sequenceEntries,
      measureSpans,
      editorRef,
      notifySelection,
      selectedSequenceRangeRef,
      clearNoMountedEditorHighlights,
    )

  // Merged purely for `Preview.tsx`'s highlight painting: an editor-mounted
  // click-and-click selection populates `noteSelectionCells`/`lyricSelectionCells` and
  // leaves `measureRangeNoteCells`/`measureRangeLyricCells` at `[]`; a
  // no-mounted-editor (Synced/shared) measure/bar-line gesture does the
  // opposite (see `useMeasureRangeSelection`'s doc comment) — the two never
  // hold cells at the same time, so concatenating is a safe union, not an
  // accidental widening of either state's own meaning.
  const selectedNoteCells = useMemo(
    () => [...noteSelectionCells, ...measureRangeNoteCells],
    [noteSelectionCells, measureRangeNoteCells],
  )
  const selectedLyricCells = useMemo(
    () => [...lyricSelectionCells, ...measureRangeLyricCells],
    [lyricSelectionCells, measureRangeLyricCells],
  )

  // The "Octave up"/"Octave down" toolbar action. A byte-offset selection
  // can't be blindly restored across this edit the way `Editor.tsx`'s
  // generic post-edit restore does for ordinary typing: a multicursor
  // selection (e.g. one from clicking a part label, which selects every
  // measure of that part in the system) can touch several notes on the same
  // line, and each note's `'`/`,` marker run grows or shrinks by its own
  // amount, so every selection after the first affected one on that line
  // would land on the wrong column if restored from its pre-edit position —
  // that's exactly what collapsed a whole-system selection down to (part
  // of) its first measure after one octave shift.
  //
  // The fix has to be synchronous: `source_edit::shift_range_octave` already
  // knows, byte-for-byte, exactly which spans it rewrote and by how much
  // each replacement grew/shrank, so it hands the new ranges straight back
  // alongside the new source (see `ShiftRangeOctaveResult` and
  // `HANDOFF-octave-toolbar-part-label-selection-bug.md`). `editorRef.current.replaceContentWithSelections`
  // then applies the new text *and* selects those ranges in one
  // uninterrupted synchronous call, so `Editor.tsx`'s own `value`-triggered
  // restore effect (which only runs on React's next commit) snapshots a
  // selection that's already correct — a no-op, not a corruption. An earlier
  // attempt re-derived the selection from `noteSpans` after the fact, but
  // that's an async worker round-trip and nothing async can reliably outrace
  // a second click firing before the correction lands.
  //
  // That `setSelections` call also fires Monaco's `onDidChangeCursorSelection`
  // synchronously, which reaches `useByteRangeSelectionCore.handleEditorSelectionChange`
  // and would normally re-derive the SVG's selected note cells by
  // overlap-testing the *new* byte ranges against `noteSpans` — but `noteSpans`
  // is only refreshed later, via the debounced `listNoteSpans` worker
  // round-trip (`useJianpuWorkerRenderRequests.ts`), so at this point it's
  // still the *old* source's spans. Overlap-testing new-source ranges against
  // old-source spans produces garbage (the SVG selection collapsing/shifting
  // to the wrong notes). Since an octave shift only rewrites `'`/`,` marker
  // runs — never adds/removes/reorders notes — the selected `(sourcePartIndex,
  // noteId)` cells themselves are unaffected by the shift, so the fix is to
  // just keep them as-is: `applyNoteSelectionSilently` re-commits the same
  // cells/runs and arms the same suppression counter `handleRangeSelect`
  // uses, so those echoed cursor-change notifications no-op instead of
  // recomputing from stale spans. Three of them fire synchronously-ish when
  // `newRanges` is non-empty (i.e. a real edit happened), not the one a
  // plain `setSelections` fires elsewhere:
  //   1. `replaceContentWithSelections`'s own `executeEdits` call (moves the
  //      cursor as a side effect of replacing the model's full text).
  //   2. `replaceContentWithSelections`'s own `setSelections` call (selects
  //      `newRanges`).
  //   3. `Editor.tsx`'s generic `value`-triggered restore effect pair (see
  //      its doc comment), which snapshots the *current* (by then already
  //      correct) selection in a `useLayoutEffect` and re-applies it in the
  //      following `useEffect` — a no-op for the *selection itself* since it
  //      just re-sets what's already there, but it still fires its own
  //      `setSelections` call, which is a third notification this counter
  //      must also swallow or `handleEditorSelectionChange` recomputes from
  //      still-stale `spans` right after the first two were correctly
  //      suppressed. This only fires when the source text actually changed
  //      (that effect pair is keyed on `value`, and React bails out of the
  //      commit — running no effects — when a state update produces the
  //      *same* string), matching exactly the `newRanges.length > 0` case.
  // When `newRanges` is empty (no note actually changed octave, e.g. every
  // selected note was already at the extreme end), only echo 1 above fires —
  // `replaceContentWithSelections` returns right after `executeEdits`,
  // skipping `setSelections`, and `handleSourceChange` doesn't produce a real
  // `value` change either. Suppressing a fixed 3 there would over-suppress
  // and silently eat the *next* real, unrelated selection-change too.
  const handleShiftSelectionOctave = useCallback(
    (ranges: EditorSelection[], delta: number) => {
      void shiftRangeOctave(ranges, delta).then(
        ({ source, ranges: newRanges }) => {
          if (noteSelectionCells.length > 0) {
            applyNoteSelectionSilently(
              noteSelectionCells,
              noteSelectionRuns,
              newRanges.length > 0 ? 3 : 1,
            )
          }
          editorRef.current?.replaceContentWithSelections(source, newRanges)
          handleSourceChange(source)
        },
      )
    },
    [
      shiftRangeOctave,
      handleSourceChange,
      editorRef,
      noteSelectionCells,
      noteSelectionRuns,
      applyNoteSelectionSilently,
    ],
  )

  const handlePlayNoteSelection = useCallback(() => {
    if (selectedNoteRangePlaybackInfo === null) return
    playNoteSelection(
      selectedNoteRangePlaybackInfo.minMeasureIndex,
      selectedNoteRangePlaybackInfo.maxMeasureIndex,
      selectedNoteRangePlaybackInfo.selectedPartNames,
      noteSelectionCells,
    )
  }, [selectedNoteRangePlaybackInfo, noteSelectionCells, playNoteSelection])

  return {
    setSelectedLineRange,
    handleSectionJump,
    sectionJumpToolbarProps,
    selectedSequenceRange,
    sequenceJumpToolbarProps,
    handleNoteRangeSelect,
    handleEditorSelectionChange,
    selectedNoteRangePlaybackInfo,
    selectedNoteCells,
    handleLyricRangeSelect,
    handleLyricEditorSelectionChange,
    selectedLyricCells,
    handleMeasureRangeSelect,
    handlePlayNoteSelection,
    handleShiftSelectionOctave,
  }
}
