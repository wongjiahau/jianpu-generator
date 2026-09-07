import type { RefObject } from 'react'
import type { NoteTimingOut, SvgDocumentOut } from '../jianpuWasm'
import type {
  Diagnostic,
  DiagnosticViewZone,
  LyricSpan,
  MeasureSpan,
  NoteSpan,
  PartDeclaration,
  PartInfo,
  SectionRange,
  SequenceEntry,
} from '../types'
import type { WorkerResponse } from '../worker/jianpu.worker'
import { handleExportMessage } from './useJianpuWorkerExportMessages'
import type {
  RangeOctaveShiftRequestTracker,
  TextRequestTracker,
} from './useJianpuWorkerTypes'

export interface WorkerMessageHandlerDeps {
  audioAvailableRef: RefObject<boolean>
  setAudioAvailable: (value: boolean) => void
  setPdfAvailable: (value: boolean) => void
  setMidiAvailable: (value: boolean) => void
  setMp3Available: (value: boolean) => void
  latestPartsIdRef: RefObject<number>
  setPartsLoading: (value: boolean) => void
  setParts: (value: PartInfo[]) => void
  setPartDeclarations: (value: PartDeclaration[]) => void
  latestUpdatePartDeclarationIdRef: RefObject<number>
  pendingPartDeclarationUpdatesRef: RefObject<
    Map<number, (source: string) => void>
  >
  latestFormatScoreIdRef: RefObject<number>
  pendingFormatScoreRequestsRef: RefObject<
    Map<number, (source: string) => void>
  >
  shiftPartOctaveTracker: TextRequestTracker
  shiftRangeOctaveTracker: RangeOctaveShiftRequestTracker
  latestPdfIdRef: RefObject<number>
  setPdfExporting: (value: boolean) => void
  activeFileRef: RefObject<string>
  enabledPartNamesRef: RefObject<string[] | undefined>
  setDiagnostics: (value: Diagnostic[]) => void
  latestSplitPdfIdRef: RefObject<number>
  setSplitPdfExporting: (value: boolean) => void
  latestMidiIdRef: RefObject<number>
  setMidiExporting: (value: boolean) => void
  latestSplitMidiIdRef: RefObject<number>
  setSplitMidiExporting: (value: boolean) => void
  latestSplitWavIdRef: RefObject<number>
  setSplitWavExporting: (value: boolean) => void
  latestMp3IdRef: RefObject<number>
  setMp3Exporting: (value: boolean) => void
  latestSplitMp3IdRef: RefObject<number>
  setSplitMp3Exporting: (value: boolean) => void
  latestRenderIdRef: RefObject<number>
  setRendering: (value: boolean) => void
  setDocuments: (value: SvgDocumentOut[]) => void
  setDiagnosticViewZones: (value: DiagnosticViewZone[]) => void
  latestAudioIdRef: RefObject<number>
  setAudioGenerating: (value: boolean) => void
  setNextWavUrl: (value: string | null) => void
  setNextMp3Url: (value: string | null) => void
  setNoteTimings: (value: NoteTimingOut[]) => void
  latestMeasureAudioIdRef: RefObject<number>
  setMeasureAudioGenerating: (value: boolean) => void
  setNextMeasureWavUrl: (
    value: string | null,
    noteTimings: NoteTimingOut[],
  ) => void
  latestHighlightRenderIdRef: RefObject<number>
  setHighlightedDocuments: (value: SvgDocumentOut[]) => void
  latestMeasureSpansIdRef: RefObject<number>
  setMeasureSpans: (value: MeasureSpan[]) => void
  latestNoteSpansIdRef: RefObject<number>
  setNoteSpans: (value: NoteSpan[]) => void
  latestLyricSpansIdRef: RefObject<number>
  setLyricSpans: (value: LyricSpan[]) => void
  setSectionRanges: (value: SectionRange[]) => void
  setSequenceEntries: (value: SequenceEntry[]) => void
  latestPreviewAudioIdRef: RefObject<number>
  currentPreviewAudioRef: RefObject<HTMLAudioElement | null>
  setPreviewAudioPlaying: (value: boolean) => void
  pendingImportsRef: RefObject<
    Map<
      number,
      { resolve: (source: string) => void; reject: (error: Error) => void }
    >
  >
  /** Opens the rename-before-download modal instead of downloading
   * immediately — see `PendingDownload` in `useJianpuWorkerTypes.ts`. */
  requestDownload: (
    url: string,
    filename: string,
    revokeOnClose: boolean,
  ) => void
}

export function createWorkerMessageHandler(deps: WorkerMessageHandlerDeps) {
  return (event: MessageEvent<WorkerResponse>) => {
    const msg = event.data
    // The twelve pdf/midi/wav/mp3 export-finished and export-failed message
    // pairs live in their own module — see `handleExportMessage`'s doc
    // comment for why.
    if (handleExportMessage(msg, deps)) return

    if (msg.type === 'ready') {
      deps.audioAvailableRef.current = msg.audioAvailable
      deps.setAudioAvailable(msg.audioAvailable)
      deps.setPdfAvailable(msg.pdfAvailable)
      deps.setMidiAvailable(msg.midiAvailable)
      deps.setMp3Available(msg.mp3Available)
      return
    }

    if (msg.type === 'parts') {
      if (msg.id !== deps.latestPartsIdRef.current) return
      deps.setPartsLoading(false)
      deps.setParts(msg.parts)
      deps.setPartDeclarations(msg.declarations)
      return
    }

    if (msg.type === 'partDeclarationUpdated') {
      if (msg.id !== deps.latestUpdatePartDeclarationIdRef.current) return
      deps.setPartDeclarations(msg.declarations)
      deps.pendingPartDeclarationUpdatesRef.current.get(msg.id)?.(msg.source)
      deps.pendingPartDeclarationUpdatesRef.current.delete(msg.id)
      return
    }

    if (msg.type === 'scoreFormatted') {
      if (msg.id !== deps.latestFormatScoreIdRef.current) return
      deps.pendingFormatScoreRequestsRef.current.get(msg.id)?.(msg.source)
      deps.pendingFormatScoreRequestsRef.current.delete(msg.id)
      return
    }

    if (msg.type === 'partOctaveShifted') {
      if (msg.id !== deps.shiftPartOctaveTracker.latestIdRef.current) return
      deps.shiftPartOctaveTracker.pendingRequestsRef.current.get(msg.id)?.(
        msg.source,
      )
      deps.shiftPartOctaveTracker.pendingRequestsRef.current.delete(msg.id)
      return
    }

    if (msg.type === 'rangeOctaveShifted') {
      if (msg.id !== deps.shiftRangeOctaveTracker.latestIdRef.current) return
      deps.shiftRangeOctaveTracker.pendingRequestsRef.current.get(msg.id)?.({
        source: msg.source,
        ranges: msg.ranges,
      })
      deps.shiftRangeOctaveTracker.pendingRequestsRef.current.delete(msg.id)
      return
    }

    if (msg.type === 'mp3') {
      if (msg.id !== deps.latestMp3IdRef.current) return
      deps.setMp3Exporting(false)
      const url = URL.createObjectURL(
        new Blob([msg.mp3], { type: 'audio/mpeg' }),
      )
      deps.setNextMp3Url(url)
      deps.setNoteTimings(msg.noteTimings)
      return
    }

    if (msg.type === 'mp3Err') {
      if (msg.id !== deps.latestMp3IdRef.current) return
      deps.setMp3Exporting(false)
      deps.setDiagnostics(msg.diagnostics)
      return
    }

    if (msg.type === 'ok') {
      if (msg.id !== deps.latestRenderIdRef.current) return
      deps.setRendering(false)
      deps.setDocuments(msg.documents)
      deps.setDiagnostics(msg.diagnostics)
      deps.setDiagnosticViewZones(msg.diagnosticViewZones)
      return
    }

    if (msg.type === 'audio') {
      if (msg.id !== deps.latestAudioIdRef.current) return
      deps.setAudioGenerating(false)
      const url = URL.createObjectURL(
        new Blob([msg.wav], { type: 'audio/wav' }),
      )
      deps.setNextWavUrl(url)
      deps.setNoteTimings(msg.noteTimings)
      return
    }

    if (msg.type === 'audioErr') {
      if (msg.id !== deps.latestAudioIdRef.current) return
      deps.setAudioGenerating(false)
      return
    }

    if (msg.type === 'measureRangeAudio') {
      if (msg.id !== deps.latestMeasureAudioIdRef.current) return
      deps.setMeasureAudioGenerating(false)
      deps.setNextMeasureWavUrl(
        URL.createObjectURL(new Blob([msg.wav], { type: 'audio/wav' })),
        msg.noteTimings,
      )
      return
    }

    if (msg.type === 'measureRangeAudioErr') {
      if (msg.id !== deps.latestMeasureAudioIdRef.current) return
      deps.setMeasureAudioGenerating(false)
      return
    }

    if (msg.type === 'highlightRangeOk') {
      if (msg.id !== deps.latestHighlightRenderIdRef.current) return
      deps.setHighlightedDocuments(msg.documents)
      return
    }

    if (msg.type === 'highlightRangeErr') {
      if (msg.id !== deps.latestHighlightRenderIdRef.current) return
      return
    }

    if (msg.type === 'measureSpans') {
      if (msg.id !== deps.latestMeasureSpansIdRef.current) return
      if (msg.status === 'ok') {
        deps.setMeasureSpans(msg.spans)
        deps.setSectionRanges(msg.sectionRanges)
        deps.setSequenceEntries(msg.sequenceEntries)
      }
      return
    }

    if (msg.type === 'noteSpans') {
      if (msg.id !== deps.latestNoteSpansIdRef.current) return
      if (msg.status === 'ok') {
        deps.setNoteSpans(msg.spans)
      }
      return
    }

    if (msg.type === 'lyricSpans') {
      if (msg.id !== deps.latestLyricSpansIdRef.current) return
      if (msg.status === 'ok') {
        deps.setLyricSpans(msg.spans)
      }
      return
    }

    if (msg.type === 'instrumentPreview' || msg.type === 'percussionPreview') {
      if (msg.id !== deps.latestPreviewAudioIdRef.current) return
      const url = URL.createObjectURL(
        new Blob([msg.wav], { type: 'audio/wav' }),
      )
      if (deps.currentPreviewAudioRef.current) {
        deps.currentPreviewAudioRef.current.pause()
      }
      const audio = new Audio(url)
      deps.currentPreviewAudioRef.current = audio
      audio.addEventListener('play', () => deps.setPreviewAudioPlaying(true))
      audio.addEventListener('ended', () => {
        deps.setPreviewAudioPlaying(false)
        URL.revokeObjectURL(url)
      })
      audio.addEventListener('pause', () => deps.setPreviewAudioPlaying(false))
      audio.play().catch(() => {})
      return
    }

    if (msg.type === 'importOk') {
      deps.pendingImportsRef.current.get(msg.id)?.resolve(msg.source)
      deps.pendingImportsRef.current.delete(msg.id)
      return
    }

    if (msg.type === 'importErr') {
      deps.pendingImportsRef.current
        .get(msg.id)
        ?.reject(
          new Error(
            'No embedded source found in this file. It may have been hand-edited or created by a different tool.',
          ),
        )
      deps.pendingImportsRef.current.delete(msg.id)
      return
    }

    if (msg.type === 'err') {
      if (msg.id !== deps.latestRenderIdRef.current) return
      deps.setRendering(false)
      deps.setDiagnostics(msg.diagnostics)
      deps.setDiagnosticViewZones(msg.diagnosticViewZones)
    }
  }
}
