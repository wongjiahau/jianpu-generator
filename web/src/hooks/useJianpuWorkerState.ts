import { useMemo, useRef, useState } from 'react'
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
import type {
  PendingDownload,
  RangeOctaveShiftRequestTracker,
  TextRequestTracker,
} from './useJianpuWorkerTypes'
import {
  disabledLyricsForRender,
  enabledPartNamesForFilename,
  enabledTracksForRender,
  mp3FilenameFromActiveFile,
  triggerAnchorDownload,
  wavFilenameFromActiveFile,
} from './workerHelpers'

/** All the plain state, refs, and derived values `useJianpuWorker` shares
 * across its sub-hooks (lifecycle, render requests, exports, measure audio,
 * instrument preview). Kept together because most of it is read and written
 * from several of those sub-hooks via refs that must stay in sync with the
 * latest render's state. */
export function useJianpuWorkerState(
  source: string,
  activeFile: string,
  disabledParts: ReadonlySet<string>,
  disabledLyrics: ReadonlySet<string>,
  soloedParts: ReadonlySet<string>,
) {
  const [parts, setParts] = useState<PartInfo[]>([])
  const [partDeclarations, setPartDeclarations] = useState<PartDeclaration[]>(
    [],
  )
  const [partsLoading, setPartsLoading] = useState(false)
  const [documents, setDocuments] = useState<SvgDocumentOut[]>([])
  const [pendingDownload, setPendingDownload] =
    useState<PendingDownload | null>(null)
  const [wavUrl, setWavUrl] = useState<string | null>(null)
  const [mp3Url, setMp3Url] = useState<string | null>(null)
  const [noteTimings, setNoteTimings] = useState<NoteTimingOut[]>([])
  const [audioAvailable, setAudioAvailable] = useState(false)
  const [pdfAvailable, setPdfAvailable] = useState(false)
  const [pdfExporting, setPdfExporting] = useState(false)
  const [splitPdfExporting, setSplitPdfExporting] = useState(false)
  const [midiAvailable, setMidiAvailable] = useState(false)
  const [midiExporting, setMidiExporting] = useState(false)
  const [splitMidiExporting, setSplitMidiExporting] = useState(false)
  const [splitWavExporting, setSplitWavExporting] = useState(false)
  const [mp3Available, setMp3Available] = useState(false)
  const [mp3Exporting, setMp3Exporting] = useState(false)
  const [splitMp3Exporting, setSplitMp3Exporting] = useState(false)
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [diagnosticViewZones, setDiagnosticViewZones] = useState<
    DiagnosticViewZone[]
  >([])
  const [rendering, setRendering] = useState(false)
  const [audioGenerating, setAudioGenerating] = useState(false)
  const [selectedMeasureRange, setSelectedMeasureRange] = useState<{
    start: number
    end: number
    /** Which measure the preview should scroll to for this selection, when
     * it differs from `start` — see `measureRangeInSpanWithReveal`. */
    revealMeasureIndex: number
    /** The exact disjoint measure ranges to highlight in the SVG preview —
     * see `measureRangeInSpanWithReveal`. */
    highlightRanges?: { start: number; end: number }[]
  } | null>(null)
  const [highlightedDocuments, setHighlightedDocuments] = useState<
    SvgDocumentOut[]
  >([])
  const [measureSpans, setMeasureSpans] = useState<MeasureSpan[]>([])
  const [noteSpans, setNoteSpans] = useState<NoteSpan[]>([])
  const [lyricSpans, setLyricSpans] = useState<LyricSpan[]>([])
  const [sectionRanges, setSectionRanges] = useState<SectionRange[]>([])
  const [sequenceEntries, setSequenceEntries] = useState<SequenceEntry[]>([])
  const highlightRenderRequestIdRef = useRef(0)
  const latestHighlightRenderIdRef = useRef(0)
  const measureSpansRequestIdRef = useRef(0)
  const latestMeasureSpansIdRef = useRef(0)
  const measureSpansRef = useRef<MeasureSpan[]>([])
  const noteSpansRequestIdRef = useRef(0)
  const latestNoteSpansIdRef = useRef(0)
  const lyricSpansRequestIdRef = useRef(0)
  const latestLyricSpansIdRef = useRef(0)
  const workerRef = useRef<Worker | null>(null)
  const wavUrlRef = useRef<string | null>(null)
  const mp3UrlRef = useRef<string | null>(null)
  const partsRequestIdRef = useRef(0)
  const updatePartDeclarationRequestIdRef = useRef(0)
  const latestUpdatePartDeclarationIdRef = useRef(0)
  const pendingPartDeclarationUpdatesRef = useRef(
    new Map<number, (source: string) => void>(),
  )
  const formatScoreRequestIdRef = useRef(0)
  const latestFormatScoreIdRef = useRef(0)
  const pendingFormatScoreRequestsRef = useRef(
    new Map<number, (source: string) => void>(),
  )
  const shiftPartOctaveTracker: TextRequestTracker = {
    requestIdRef: useRef(0),
    latestIdRef: useRef(0),
    pendingRequestsRef: useRef(new Map<number, (source: string) => void>()),
  }
  const shiftRangeOctaveTracker: RangeOctaveShiftRequestTracker = {
    requestIdRef: useRef(0),
    latestIdRef: useRef(0),
    pendingRequestsRef: useRef(
      new Map<
        number,
        (result: {
          source: string
          ranges: { start: number; end: number }[]
        }) => void
      >(),
    ),
  }
  const importRequestIdRef = useRef(0)
  const pendingImportsRef = useRef(
    new Map<
      number,
      { resolve: (source: string) => void; reject: (error: Error) => void }
    >(),
  )
  const renderRequestIdRef = useRef(0)
  const audioRequestIdRef = useRef(0)
  const pdfRequestIdRef = useRef(0)
  const splitPdfRequestIdRef = useRef(0)
  const midiRequestIdRef = useRef(0)
  const splitMidiRequestIdRef = useRef(0)
  const splitWavRequestIdRef = useRef(0)
  const mp3RequestIdRef = useRef(0)
  const splitMp3RequestIdRef = useRef(0)
  const latestPartsIdRef = useRef(0)
  const latestRenderIdRef = useRef(0)
  const latestAudioIdRef = useRef(0)
  const latestPdfIdRef = useRef(0)
  const latestSplitPdfIdRef = useRef(0)
  const latestMidiIdRef = useRef(0)
  const latestSplitMidiIdRef = useRef(0)
  const latestSplitWavIdRef = useRef(0)
  const latestMp3IdRef = useRef(0)
  const latestSplitMp3IdRef = useRef(0)
  const sourceRef = useRef(source)
  const activeFileRef = useRef(activeFile)
  const enabledTracksRef = useRef<string[] | undefined>(undefined)
  const enabledPartNamesRef = useRef<string[] | undefined>(undefined)
  const disabledLyricsRef = useRef<string[] | undefined>(undefined)
  const audioAvailableRef = useRef(false)
  const cursorOffsetTimerRef = useRef<number | null>(null)
  const lastSelectionRef = useRef<{
    start: number
    end: number
    isEmpty: boolean
    revealLine: number
  } | null>(null)

  /** Opens the rename-before-download modal for `url`/`filename` instead of
   * downloading immediately — see `PendingDownload`'s doc comment for what
   * `revokeOnClose` controls. */
  function requestDownload(
    url: string,
    filename: string,
    revokeOnClose: boolean,
  ) {
    setPendingDownload({ url, filename, revokeOnClose })
  }

  /** User confirmed the modal (Download button or Enter) — fires the
   * download under the (possibly edited) `filename`, then revokes the
   * object URL and clears the pending state. */
  function confirmPendingDownload(filename: string) {
    if (!pendingDownload) return
    triggerAnchorDownload(pendingDownload.url, filename)
    if (pendingDownload.revokeOnClose) URL.revokeObjectURL(pendingDownload.url)
    setPendingDownload(null)
  }

  /** User cancelled the modal (Cancel/Escape/overlay click) — no download,
   * just cleanup. */
  function cancelPendingDownload() {
    if (!pendingDownload) return
    if (pendingDownload.revokeOnClose) URL.revokeObjectURL(pendingDownload.url)
    setPendingDownload(null)
  }

  const effectiveDisabledParts = useMemo(() => {
    if (soloedParts.size === 0) return disabledParts
    return new Set(
      parts
        .map((part) => part.abbreviation)
        .filter((abbr) => !soloedParts.has(abbr)),
    )
  }, [soloedParts, parts, disabledParts])

  const enabledTracks = useMemo(
    () => enabledTracksForRender(parts, effectiveDisabledParts),
    [parts, effectiveDisabledParts],
  )
  const enabledPartNames = useMemo(
    () => enabledPartNamesForFilename(parts, effectiveDisabledParts),
    [parts, effectiveDisabledParts],
  )
  const disabledLyricsTracks = useMemo(
    () => disabledLyricsForRender(parts, disabledLyrics),
    [parts, disabledLyrics],
  )
  const wavFilename = useMemo(
    () => wavFilenameFromActiveFile(activeFile, enabledPartNames),
    [activeFile, enabledPartNames],
  )
  const mp3Filename = useMemo(
    () => mp3FilenameFromActiveFile(activeFile, enabledPartNames),
    [activeFile, enabledPartNames],
  )

  sourceRef.current = source
  activeFileRef.current = activeFile
  enabledTracksRef.current = enabledTracks
  enabledPartNamesRef.current = enabledPartNames
  disabledLyricsRef.current = disabledLyricsTracks
  measureSpansRef.current = measureSpans

  return {
    parts,
    setParts,
    partDeclarations,
    setPartDeclarations,
    partsLoading,
    setPartsLoading,
    documents,
    setDocuments,
    pendingDownload,
    requestDownload,
    confirmPendingDownload,
    cancelPendingDownload,
    wavUrl,
    setWavUrl,
    mp3Url,
    setMp3Url,
    noteTimings,
    setNoteTimings,
    audioAvailable,
    setAudioAvailable,
    pdfAvailable,
    setPdfAvailable,
    pdfExporting,
    setPdfExporting,
    splitPdfExporting,
    setSplitPdfExporting,
    midiAvailable,
    setMidiAvailable,
    midiExporting,
    setMidiExporting,
    splitMidiExporting,
    setSplitMidiExporting,
    splitWavExporting,
    setSplitWavExporting,
    mp3Available,
    setMp3Available,
    mp3Exporting,
    setMp3Exporting,
    splitMp3Exporting,
    setSplitMp3Exporting,
    diagnostics,
    setDiagnostics,
    diagnosticViewZones,
    setDiagnosticViewZones,
    rendering,
    setRendering,
    audioGenerating,
    setAudioGenerating,
    selectedMeasureRange,
    setSelectedMeasureRange,
    highlightedDocuments,
    setHighlightedDocuments,
    measureSpans,
    setMeasureSpans,
    noteSpans,
    setNoteSpans,
    lyricSpans,
    setLyricSpans,
    sectionRanges,
    setSectionRanges,
    sequenceEntries,
    setSequenceEntries,
    highlightRenderRequestIdRef,
    latestHighlightRenderIdRef,
    measureSpansRequestIdRef,
    latestMeasureSpansIdRef,
    measureSpansRef,
    noteSpansRequestIdRef,
    latestNoteSpansIdRef,
    lyricSpansRequestIdRef,
    latestLyricSpansIdRef,
    workerRef,
    wavUrlRef,
    mp3UrlRef,
    partsRequestIdRef,
    updatePartDeclarationRequestIdRef,
    latestUpdatePartDeclarationIdRef,
    pendingPartDeclarationUpdatesRef,
    formatScoreRequestIdRef,
    latestFormatScoreIdRef,
    pendingFormatScoreRequestsRef,
    shiftPartOctaveTracker,
    shiftRangeOctaveTracker,
    importRequestIdRef,
    pendingImportsRef,
    renderRequestIdRef,
    audioRequestIdRef,
    pdfRequestIdRef,
    splitPdfRequestIdRef,
    midiRequestIdRef,
    splitMidiRequestIdRef,
    splitWavRequestIdRef,
    mp3RequestIdRef,
    splitMp3RequestIdRef,
    latestPartsIdRef,
    latestRenderIdRef,
    latestAudioIdRef,
    latestPdfIdRef,
    latestSplitPdfIdRef,
    latestMidiIdRef,
    latestSplitMidiIdRef,
    latestSplitWavIdRef,
    latestMp3IdRef,
    latestSplitMp3IdRef,
    sourceRef,
    activeFileRef,
    enabledTracksRef,
    enabledPartNamesRef,
    disabledLyricsRef,
    audioAvailableRef,
    cursorOffsetTimerRef,
    lastSelectionRef,
    enabledTracks,
    enabledPartNames,
    disabledLyricsTracks,
    wavFilename,
    mp3Filename,
  }
}
