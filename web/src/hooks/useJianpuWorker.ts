import type { RefObject } from 'react'
import { useJianpuWorkerActions } from './useJianpuWorkerActions'
import { useJianpuWorkerState } from './useJianpuWorkerState'
import type { JianpuWorkerState } from './useJianpuWorkerTypes'

export type { JianpuWorkerState } from './useJianpuWorkerTypes'

export function useJianpuWorker(
  source: string,
  disabledParts: ReadonlySet<string>,
  disabledLyrics: ReadonlySet<string>,
  soloedParts: ReadonlySet<string>,
  activeFile: string,
  soundfontBytes: Uint8Array | null,
  fontBytes: { sc: Uint8Array; tc: Uint8Array; mono: Uint8Array } | null,
  /**
   * Owned by the caller (`useAppController`, which also feeds it to its own
   * `useSequenceNavigation` call) rather than this hook, since
   * `useMeasureAudioPlayback` below needs it before `useSequenceNavigation`
   * can run — that hook needs `notifySelection`, which this hook only
   * produces further down. See `useSequenceNavigation`'s matching parameter
   * doc comment.
   */
  selectedSequenceRangeRef: RefObject<{
    start: number
    end: number
    entryStartIndex: number
    entryEndIndex: number
  } | null>,
  debounceMs = 300,
): JianpuWorkerState {
  const state = useJianpuWorkerState(
    source,
    activeFile,
    disabledParts,
    disabledLyrics,
    soloedParts,
  )
  const {
    parts,
    partDeclarations,
    partsLoading,
    documents,
    pendingDownload,
    requestDownload,
    confirmPendingDownload,
    cancelPendingDownload,
    wavUrl,
    wavFilename,
    mp3Url,
    mp3Filename,
    noteTimings,
    audioAvailable,
    pdfAvailable,
    pdfExporting,
    splitPdfExporting,
    midiAvailable,
    midiExporting,
    splitMidiExporting,
    splitWavExporting,
    mp3Available,
    mp3Exporting,
    splitMp3Exporting,
    diagnostics,
    diagnosticViewZones,
    rendering,
    audioGenerating,
    selectedMeasureRange,
    highlightedDocuments,
    measureSpans,
    noteSpans,
    lyricSpans,
    sectionRanges,
    sequenceEntries,
    enabledTracks,
  } = state

  const actions = useJianpuWorkerActions({
    state,
    selectedSequenceRangeRef,
    source,
    activeFile,
    soundfontBytes,
    fontBytes,
    debounceMs,
  })

  return {
    parts,
    partDeclarations,
    partsLoading,
    documents,
    pendingDownload,
    requestDownload,
    confirmPendingDownload,
    cancelPendingDownload,
    wavUrl,
    wavFilename,
    mp3Url,
    mp3Filename,
    noteTimings,
    audioAvailable,
    pdfAvailable,
    pdfExporting,
    splitPdfExporting,
    midiAvailable,
    midiExporting,
    splitMidiExporting,
    splitWavExporting,
    mp3Available,
    mp3Exporting,
    splitMp3Exporting,
    diagnostics,
    diagnosticViewZones,
    rendering,
    audioGenerating,
    exportPdf: actions.exportPdf,
    exportSplitPdf: actions.exportSplitPdf,
    exportMidi: actions.exportMidi,
    exportSplitMidi: actions.exportSplitMidi,
    exportSplitWav: actions.exportSplitWav,
    exportMp3: actions.exportMp3,
    exportSplitMp3: actions.exportSplitMp3,
    generateFullAudio: actions.generateFullAudio,
    selectedMeasureRange,
    measureAudioGenerating: actions.measureAudioGenerating,
    measureAudioPlaying: actions.measureAudioPlaying,
    measureAudioNoteTimings: actions.measureAudioNoteTimings,
    measureAudioElement: actions.measureAudioElement,
    notifySelection: actions.notifySelection,
    playSelectedMeasures: actions.playSelectedMeasures,
    playFromCurrentMeasure: actions.playFromCurrentMeasure,
    playNoteSelection: actions.playNoteSelection,
    playAll: actions.playAll,
    stopMeasurePlayback: actions.stopMeasurePlayback,
    highlightedDocuments,
    measureSpans,
    noteSpans,
    lyricSpans,
    sectionRanges,
    sequenceEntries,
    enabledTracks,
    previewInstrument: actions.previewInstrument,
    previewPercussion: actions.previewPercussion,
    stopPreviewInstrument: actions.stopPreviewInstrument,
    previewAudioPlaying: actions.previewAudioPlaying,
    updatePartDeclaration: actions.updatePartDeclaration,
    formatScore: actions.formatScore,
    shiftPartOctave: actions.shiftPartOctave,
    shiftRangeOctave: actions.shiftRangeOctave,
    importFromFile: actions.importFromFile,
  }
}
