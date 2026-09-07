import type { RefObject } from 'react'
import { useCallback, useEffect, useRef } from 'react'
import type { SvgDocumentOut } from '../jianpuWasm'
import type { Diagnostic, MeasureSpan } from '../types'
import type { WorkerRequest } from '../worker/jianpu.worker'
import { measureRangeInSpanWithReveal } from './workerHelpers'

interface UseJianpuWorkerRenderRequestsParams {
  workerRef: RefObject<Worker | null>
  sourceRef: RefObject<string>
  source: string
  activeFile: string
  debounceMs: number
  enabledTracks: string[] | undefined
  disabledLyricsTracks: string[] | undefined
  setDocuments: (value: SvgDocumentOut[]) => void
  setNextWavUrl: (value: string | null) => void
  setNextMp3Url: (value: string | null) => void
  setDiagnostics: (value: Diagnostic[]) => void
  setPartsLoading: (value: boolean) => void
  partsRequestIdRef: RefObject<number>
  latestPartsIdRef: RefObject<number>
  setRendering: (value: boolean) => void
  renderRequestIdRef: RefObject<number>
  latestRenderIdRef: RefObject<number>
  selectedMeasureRange: {
    start: number
    end: number
    revealMeasureIndex: number
    highlightRanges?: { start: number; end: number }[]
  } | null
  setSelectedMeasureRange: (
    value: {
      start: number
      end: number
      revealMeasureIndex: number
      highlightRanges?: { start: number; end: number }[]
    } | null,
  ) => void
  setHighlightedDocuments: (value: SvgDocumentOut[]) => void
  highlightRenderRequestIdRef: RefObject<number>
  latestHighlightRenderIdRef: RefObject<number>
  measureSpans: MeasureSpan[]
  measureSpansRef: RefObject<MeasureSpan[]>
  measureSpansRequestIdRef: RefObject<number>
  latestMeasureSpansIdRef: RefObject<number>
  noteSpansRequestIdRef: RefObject<number>
  latestNoteSpansIdRef: RefObject<number>
  lyricSpansRequestIdRef: RefObject<number>
  latestLyricSpansIdRef: RefObject<number>
  cursorOffsetTimerRef: RefObject<number | null>
  lastSelectionRef: RefObject<{
    start: number
    end: number
    isEmpty: boolean
    revealLine: number
    measureRanges?: { start: number; end: number }[]
  } | null>
}

/** Debounced worker requests that keep parts, rendered documents, highlighted documents and
 * measure spans in sync with the current source, plus the selection-to-measure-range mapping. */
export function useJianpuWorkerRenderRequests({
  workerRef,
  sourceRef,
  source,
  activeFile,
  debounceMs,
  enabledTracks,
  disabledLyricsTracks,
  setDocuments,
  setNextWavUrl,
  setNextMp3Url,
  setDiagnostics,
  setPartsLoading,
  partsRequestIdRef,
  latestPartsIdRef,
  setRendering,
  renderRequestIdRef,
  latestRenderIdRef,
  selectedMeasureRange,
  setSelectedMeasureRange,
  setHighlightedDocuments,
  highlightRenderRequestIdRef,
  latestHighlightRenderIdRef,
  measureSpans,
  measureSpansRef,
  measureSpansRequestIdRef,
  latestMeasureSpansIdRef,
  noteSpansRequestIdRef,
  latestNoteSpansIdRef,
  lyricSpansRequestIdRef,
  latestLyricSpansIdRef,
  cursorOffsetTimerRef,
  lastSelectionRef,
}: UseJianpuWorkerRenderRequestsParams) {
  // Tracks whether the selection behind the current `selectedMeasureRange`
  // is a bare caret (0-length) rather than a highlighted range. Kept
  // separate from `selectedMeasureRange` itself, which other consumers
  // (the play-selection button, the selected-range badge) still need
  // populated for a real text selection — only the preview's amber
  // measure-background rect is gated on this.
  const measureRangeIsCaretOnlyRef = useRef(true)

  // biome-ignore lint/correctness/useExhaustiveDependencies: activeFile is intentional trigger
  useEffect(() => {
    setDocuments([])
    setNextWavUrl(null)
    setNextMp3Url(null)
    setDiagnostics([])
  }, [activeFile, setNextWavUrl, setNextMp3Url])

  // biome-ignore lint/correctness/useExhaustiveDependencies: source is intentional trigger
  useEffect(() => {
    setSelectedMeasureRange(null)
  }, [source])

  // biome-ignore lint/correctness/useExhaustiveDependencies: workerRef/partsRequestIdRef/latestPartsIdRef are stable refs passed in as params
  useEffect(() => {
    const worker = workerRef.current
    if (!worker) return

    const id = ++partsRequestIdRef.current
    latestPartsIdRef.current = id
    setPartsLoading(true)

    const timer = window.setTimeout(() => {
      const payload: WorkerRequest = { type: 'listParts', source, id }
      worker.postMessage(payload)
    }, debounceMs)

    return () => window.clearTimeout(timer)
  }, [source, debounceMs])

  // biome-ignore lint/correctness/useExhaustiveDependencies: activeFile triggers re-render after rename (content unchanged but activeFile changes); workerRef/renderRequestIdRef/latestRenderIdRef are stable refs passed in as params
  useEffect(() => {
    const worker = workerRef.current
    if (!worker) return

    const id = ++renderRequestIdRef.current
    latestRenderIdRef.current = id
    setRendering(true)

    const payload: WorkerRequest = {
      type: 'render',
      source,
      id,
      enabledTracks,
      disabledLyrics: disabledLyricsTracks,
    }
    worker.postMessage(payload)
  }, [source, activeFile, enabledTracks, disabledLyricsTracks])

  // biome-ignore lint/correctness/useExhaustiveDependencies: lastSelectionRef/cursorOffsetTimerRef/measureSpansRef are stable refs passed in as params
  const notifySelection = useCallback(
    (
      startLine: number,
      endLine: number,
      isEmpty: boolean,
      revealLine: number = startLine,
      measureRanges?: { start: number; end: number }[],
    ) => {
      lastSelectionRef.current = {
        start: startLine,
        end: endLine,
        isEmpty,
        revealLine,
        measureRanges,
      }
      if (cursorOffsetTimerRef.current !== null) {
        window.clearTimeout(cursorOffsetTimerRef.current)
      }
      cursorOffsetTimerRef.current = window.setTimeout(() => {
        cursorOffsetTimerRef.current = null
        measureRangeIsCaretOnlyRef.current = isEmpty
        setSelectedMeasureRange(
          measureRangeInSpanWithReveal(
            measureSpansRef.current,
            startLine,
            endLine,
            revealLine,
            measureRanges,
          ),
        )
      }, debounceMs)
    },
    [debounceMs],
  )

  // biome-ignore lint/correctness/useExhaustiveDependencies: lastSelectionRef is a stable ref passed in as a param
  useEffect(() => {
    const sel = lastSelectionRef.current
    if (!sel) return
    measureRangeIsCaretOnlyRef.current = sel.isEmpty
    setSelectedMeasureRange(
      measureRangeInSpanWithReveal(
        measureSpans,
        sel.start,
        sel.end,
        sel.revealLine,
        sel.measureRanges,
      ),
    )
  }, [measureSpans])

  // biome-ignore lint/correctness/useExhaustiveDependencies: workerRef/sourceRef/highlightRenderRequestIdRef/latestHighlightRenderIdRef are stable refs passed in as params
  useEffect(() => {
    // A `# sequence` chain selection carries its own exact disjoint
    // highlight ranges (`highlightRanges`, from `computeSequenceSelectionMeasureRanges`)
    // — bypassing the caret-only gate below entirely, since it's the only
    // caller that ever needs a real (non-caret) range highlighted. Every
    // other range selection (section jumps, note/lyric range-select, plain
    // Monaco text selection) keeps today's behavior: only a bare caret gets
    // the amber measure-background highlight; a real selection still
    // populates `selectedMeasureRange` for playback/badge purposes, but
    // shouldn't paint this background.
    const highlightRanges =
      selectedMeasureRange?.highlightRanges ??
      (measureRangeIsCaretOnlyRef.current && selectedMeasureRange
        ? [{ start: selectedMeasureRange.start, end: selectedMeasureRange.end }]
        : null)
    if (!highlightRanges) {
      setHighlightedDocuments([])
      return
    }
    const worker = workerRef.current
    if (!worker) return
    const id = ++highlightRenderRequestIdRef.current
    latestHighlightRenderIdRef.current = id
    worker.postMessage({
      type: 'renderWithHighlightRange',
      source: sourceRef.current,
      id,
      ranges: highlightRanges,
      enabledTracks,
      disabledLyrics: disabledLyricsTracks,
    } satisfies WorkerRequest)
  }, [selectedMeasureRange, enabledTracks, disabledLyricsTracks])

  // biome-ignore lint/correctness/useExhaustiveDependencies: workerRef/measureSpansRequestIdRef/latestMeasureSpansIdRef are stable refs passed in as params
  useEffect(() => {
    const worker = workerRef.current
    if (!worker) return

    const id = ++measureSpansRequestIdRef.current
    latestMeasureSpansIdRef.current = id

    const timer = window.setTimeout(() => {
      worker.postMessage({
        type: 'listMeasureSpans',
        source,
        id,
      } satisfies WorkerRequest)
    }, debounceMs)

    return () => window.clearTimeout(timer)
  }, [source, debounceMs])

  // biome-ignore lint/correctness/useExhaustiveDependencies: workerRef/noteSpansRequestIdRef/latestNoteSpansIdRef are stable refs passed in as params
  useEffect(() => {
    const worker = workerRef.current
    if (!worker) return

    const id = ++noteSpansRequestIdRef.current
    latestNoteSpansIdRef.current = id

    const timer = window.setTimeout(() => {
      worker.postMessage({
        type: 'listNoteSpans',
        source,
        id,
        enabledTracks,
      } satisfies WorkerRequest)
    }, debounceMs)

    return () => window.clearTimeout(timer)
  }, [source, debounceMs, enabledTracks])

  // biome-ignore lint/correctness/useExhaustiveDependencies: workerRef/lyricSpansRequestIdRef/latestLyricSpansIdRef are stable refs passed in as params
  useEffect(() => {
    const worker = workerRef.current
    if (!worker) return

    const id = ++lyricSpansRequestIdRef.current
    latestLyricSpansIdRef.current = id

    const timer = window.setTimeout(() => {
      worker.postMessage({
        type: 'listLyricSpans',
        source,
        id,
        enabledTracks,
      } satisfies WorkerRequest)
    }, debounceMs)

    return () => window.clearTimeout(timer)
  }, [source, debounceMs, enabledTracks])

  return { notifySelection }
}
