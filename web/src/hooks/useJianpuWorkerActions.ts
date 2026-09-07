import type { RefObject } from 'react'
import { useInstrumentPreview } from './useInstrumentPreview'
import { useJianpuWorkerAudioActions } from './useJianpuWorkerAudioActions'
import { useJianpuWorkerExports } from './useJianpuWorkerExports'
import { useJianpuWorkerFormat } from './useJianpuWorkerFormat'
import { useJianpuWorkerImport } from './useJianpuWorkerImport'
import { useJianpuWorkerLifecycle } from './useJianpuWorkerLifecycle'
import { useJianpuWorkerPartDeclaration } from './useJianpuWorkerPartDeclaration'
import { useJianpuWorkerRenderRequests } from './useJianpuWorkerRenderRequests'
import { useJianpuWorkerShiftOctave } from './useJianpuWorkerShiftOctave'
import { useJianpuWorkerShiftRangeOctave } from './useJianpuWorkerShiftRangeOctave'
import type { useJianpuWorkerState } from './useJianpuWorkerState'
import { useMeasureAudioPlayback } from './useMeasureAudioPlayback'

interface UseJianpuWorkerActionsParams {
  state: ReturnType<typeof useJianpuWorkerState>
  selectedSequenceRangeRef: RefObject<{
    start: number
    end: number
    entryStartIndex: number
    entryEndIndex: number
  } | null>
  source: string
  activeFile: string
  soundfontBytes: Uint8Array | null
  fontBytes: { sc: Uint8Array; tc: Uint8Array; mono: Uint8Array } | null
  debounceMs: number
}

/**
 * Wires every `useJianpuWorker` sub-hook that isn't plain state
 * (`useJianpuWorkerState`) — audio/measure/instrument playback, worker
 * lifecycle callbacks, render requests, exports, and the one-off
 * part-declaration/format/shift-octave/import actions — and returns
 * everything `useJianpuWorker` needs beyond `state` itself to assemble its
 * public return value.
 *
 * Split out of `useJianpuWorker` purely to stay under the 400-line file
 * cap; the two only make sense read together, and reads `state`'s fields
 * directly (rather than destructuring them into locals) so nothing here
 * shadows or drifts from the single `useJianpuWorkerState` call in the
 * caller.
 */
export function useJianpuWorkerActions({
  state,
  selectedSequenceRangeRef,
  source,
  activeFile,
  soundfontBytes,
  fontBytes,
  debounceMs,
}: UseJianpuWorkerActionsParams) {
  const { setNextWavUrl, setNextMp3Url, generateFullAudio } =
    useJianpuWorkerAudioActions({
      workerRef: state.workerRef,
      sourceRef: state.sourceRef,
      enabledTracksRef: state.enabledTracksRef,
      wavUrlRef: state.wavUrlRef,
      setWavUrl: state.setWavUrl,
      mp3UrlRef: state.mp3UrlRef,
      setMp3Url: state.setMp3Url,
      audioGenerating: state.audioGenerating,
      setAudioGenerating: state.setAudioGenerating,
      audioRequestIdRef: state.audioRequestIdRef,
      latestAudioIdRef: state.latestAudioIdRef,
    })

  const {
    measureAudioGenerating,
    setMeasureAudioGenerating,
    measureAudioPlaying,
    measureAudioNoteTimings,
    measureAudioElement,
    setNextMeasureWavUrl,
    stopMeasurePlayback,
    playSelectedMeasures,
    playFromCurrentMeasure,
    playNoteSelection,
    playAll,
    latestMeasureAudioIdRef,
    measureWavUrlRef,
  } = useMeasureAudioPlayback({
    workerRef: state.workerRef,
    sourceRef: state.sourceRef,
    enabledTracksRef: state.enabledTracksRef,
    selectedMeasureRange: state.selectedMeasureRange,
    selectedSequenceRangeRef,
    totalMeasures: state.measureSpans.length,
  })

  const {
    previewAudioPlaying,
    setPreviewAudioPlaying,
    previewInstrument,
    previewPercussion,
    stopPreviewInstrument,
    latestPreviewAudioIdRef,
    currentPreviewAudioRef,
  } = useInstrumentPreview({ workerRef: state.workerRef })

  useJianpuWorkerLifecycle({
    workerRef: state.workerRef,
    wavUrlRef: state.wavUrlRef,
    mp3UrlRef: state.mp3UrlRef,
    measureWavUrlRef,
    cursorOffsetTimerRef: state.cursorOffsetTimerRef,
    soundfontBytes,
    fontBytes,
    audioAvailableRef: state.audioAvailableRef,
    setAudioAvailable: state.setAudioAvailable,
    setPdfAvailable: state.setPdfAvailable,
    setMidiAvailable: state.setMidiAvailable,
    setMp3Available: state.setMp3Available,
    latestPartsIdRef: state.latestPartsIdRef,
    setPartsLoading: state.setPartsLoading,
    setParts: state.setParts,
    setPartDeclarations: state.setPartDeclarations,
    latestUpdatePartDeclarationIdRef: state.latestUpdatePartDeclarationIdRef,
    pendingPartDeclarationUpdatesRef: state.pendingPartDeclarationUpdatesRef,
    latestFormatScoreIdRef: state.latestFormatScoreIdRef,
    pendingFormatScoreRequestsRef: state.pendingFormatScoreRequestsRef,
    shiftPartOctaveTracker: state.shiftPartOctaveTracker,
    shiftRangeOctaveTracker: state.shiftRangeOctaveTracker,
    latestPdfIdRef: state.latestPdfIdRef,
    setPdfExporting: state.setPdfExporting,
    activeFileRef: state.activeFileRef,
    enabledPartNamesRef: state.enabledPartNamesRef,
    setDiagnostics: state.setDiagnostics,
    latestSplitPdfIdRef: state.latestSplitPdfIdRef,
    setSplitPdfExporting: state.setSplitPdfExporting,
    latestMidiIdRef: state.latestMidiIdRef,
    setMidiExporting: state.setMidiExporting,
    latestSplitMidiIdRef: state.latestSplitMidiIdRef,
    setSplitMidiExporting: state.setSplitMidiExporting,
    latestSplitWavIdRef: state.latestSplitWavIdRef,
    setSplitWavExporting: state.setSplitWavExporting,
    latestMp3IdRef: state.latestMp3IdRef,
    setMp3Exporting: state.setMp3Exporting,
    latestSplitMp3IdRef: state.latestSplitMp3IdRef,
    setSplitMp3Exporting: state.setSplitMp3Exporting,
    latestRenderIdRef: state.latestRenderIdRef,
    setRendering: state.setRendering,
    setDocuments: state.setDocuments,
    setDiagnosticViewZones: state.setDiagnosticViewZones,
    latestAudioIdRef: state.latestAudioIdRef,
    setAudioGenerating: state.setAudioGenerating,
    setNextWavUrl,
    setNextMp3Url,
    setNoteTimings: state.setNoteTimings,
    latestMeasureAudioIdRef,
    setMeasureAudioGenerating,
    setNextMeasureWavUrl,
    latestHighlightRenderIdRef: state.latestHighlightRenderIdRef,
    setHighlightedDocuments: state.setHighlightedDocuments,
    latestMeasureSpansIdRef: state.latestMeasureSpansIdRef,
    setMeasureSpans: state.setMeasureSpans,
    latestNoteSpansIdRef: state.latestNoteSpansIdRef,
    setNoteSpans: state.setNoteSpans,
    latestLyricSpansIdRef: state.latestLyricSpansIdRef,
    setLyricSpans: state.setLyricSpans,
    setSectionRanges: state.setSectionRanges,
    setSequenceEntries: state.setSequenceEntries,
    latestPreviewAudioIdRef,
    currentPreviewAudioRef,
    setPreviewAudioPlaying,
    pendingImportsRef: state.pendingImportsRef,
    requestDownload: state.requestDownload,
  })

  const { notifySelection } = useJianpuWorkerRenderRequests({
    workerRef: state.workerRef,
    sourceRef: state.sourceRef,
    source,
    activeFile,
    debounceMs,
    cursorOffsetTimerRef: state.cursorOffsetTimerRef,
    enabledTracks: state.enabledTracks,
    disabledLyricsTracks: state.disabledLyricsTracks,
    setDocuments: state.setDocuments,
    setNextWavUrl,
    setNextMp3Url,
    setDiagnostics: state.setDiagnostics,
    setPartsLoading: state.setPartsLoading,
    partsRequestIdRef: state.partsRequestIdRef,
    latestPartsIdRef: state.latestPartsIdRef,
    setRendering: state.setRendering,
    renderRequestIdRef: state.renderRequestIdRef,
    latestRenderIdRef: state.latestRenderIdRef,
    selectedMeasureRange: state.selectedMeasureRange,
    setSelectedMeasureRange: state.setSelectedMeasureRange,
    setHighlightedDocuments: state.setHighlightedDocuments,
    highlightRenderRequestIdRef: state.highlightRenderRequestIdRef,
    latestHighlightRenderIdRef: state.latestHighlightRenderIdRef,
    measureSpans: state.measureSpans,
    measureSpansRef: state.measureSpansRef,
    measureSpansRequestIdRef: state.measureSpansRequestIdRef,
    latestMeasureSpansIdRef: state.latestMeasureSpansIdRef,
    noteSpansRequestIdRef: state.noteSpansRequestIdRef,
    latestNoteSpansIdRef: state.latestNoteSpansIdRef,
    lyricSpansRequestIdRef: state.lyricSpansRequestIdRef,
    latestLyricSpansIdRef: state.latestLyricSpansIdRef,
    lastSelectionRef: state.lastSelectionRef,
  })

  const {
    exportPdf,
    exportSplitPdf,
    exportMidi,
    exportSplitMidi,
    exportSplitWav,
    exportMp3,
    exportSplitMp3,
  } = useJianpuWorkerExports({
    workerRef: state.workerRef,
    sourceRef: state.sourceRef,
    activeFileRef: state.activeFileRef,
    enabledTracksRef: state.enabledTracksRef,
    disabledLyricsRef: state.disabledLyricsRef,
    pdfExporting: state.pdfExporting,
    splitPdfExporting: state.splitPdfExporting,
    midiExporting: state.midiExporting,
    splitMidiExporting: state.splitMidiExporting,
    splitWavExporting: state.splitWavExporting,
    mp3Exporting: state.mp3Exporting,
    splitMp3Exporting: state.splitMp3Exporting,
    setPdfExporting: state.setPdfExporting,
    setSplitPdfExporting: state.setSplitPdfExporting,
    setMidiExporting: state.setMidiExporting,
    setSplitMidiExporting: state.setSplitMidiExporting,
    setSplitWavExporting: state.setSplitWavExporting,
    setMp3Exporting: state.setMp3Exporting,
    setSplitMp3Exporting: state.setSplitMp3Exporting,
    pdfRequestIdRef: state.pdfRequestIdRef,
    latestPdfIdRef: state.latestPdfIdRef,
    splitPdfRequestIdRef: state.splitPdfRequestIdRef,
    latestSplitPdfIdRef: state.latestSplitPdfIdRef,
    midiRequestIdRef: state.midiRequestIdRef,
    latestMidiIdRef: state.latestMidiIdRef,
    splitMidiRequestIdRef: state.splitMidiRequestIdRef,
    latestSplitMidiIdRef: state.latestSplitMidiIdRef,
    splitWavRequestIdRef: state.splitWavRequestIdRef,
    latestSplitWavIdRef: state.latestSplitWavIdRef,
    mp3RequestIdRef: state.mp3RequestIdRef,
    latestMp3IdRef: state.latestMp3IdRef,
    splitMp3RequestIdRef: state.splitMp3RequestIdRef,
    latestSplitMp3IdRef: state.latestSplitMp3IdRef,
  })

  const { updatePartDeclaration } = useJianpuWorkerPartDeclaration({
    workerRef: state.workerRef,
    sourceRef: state.sourceRef,
    updatePartDeclarationRequestIdRef: state.updatePartDeclarationRequestIdRef,
    latestUpdatePartDeclarationIdRef: state.latestUpdatePartDeclarationIdRef,
    pendingPartDeclarationUpdatesRef: state.pendingPartDeclarationUpdatesRef,
  })

  const { formatScore } = useJianpuWorkerFormat({
    workerRef: state.workerRef,
    formatScoreRequestIdRef: state.formatScoreRequestIdRef,
    latestFormatScoreIdRef: state.latestFormatScoreIdRef,
    pendingFormatScoreRequestsRef: state.pendingFormatScoreRequestsRef,
  })

  const { shiftPartOctave } = useJianpuWorkerShiftOctave({
    workerRef: state.workerRef,
    sourceRef: state.sourceRef,
    shiftPartOctaveTracker: state.shiftPartOctaveTracker,
  })

  const { shiftRangeOctave } = useJianpuWorkerShiftRangeOctave({
    workerRef: state.workerRef,
    sourceRef: state.sourceRef,
    shiftRangeOctaveTracker: state.shiftRangeOctaveTracker,
  })

  const { importFromFile } = useJianpuWorkerImport({
    workerRef: state.workerRef,
    importRequestIdRef: state.importRequestIdRef,
    pendingImportsRef: state.pendingImportsRef,
  })

  return {
    generateFullAudio,
    measureAudioGenerating,
    measureAudioPlaying,
    measureAudioNoteTimings,
    measureAudioElement,
    stopMeasurePlayback,
    playSelectedMeasures,
    playFromCurrentMeasure,
    playNoteSelection,
    playAll,
    notifySelection,
    exportPdf,
    exportSplitPdf,
    exportMidi,
    exportSplitMidi,
    exportSplitWav,
    exportMp3,
    exportSplitMp3,
    updatePartDeclaration,
    formatScore,
    shiftPartOctave,
    shiftRangeOctave,
    importFromFile,
    previewInstrument,
    previewPercussion,
    stopPreviewInstrument,
    previewAudioPlaying,
  }
}
