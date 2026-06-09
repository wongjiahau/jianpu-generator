import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { Diagnostic, PartInfo } from '../types'
import type { WorkerRequest, WorkerResponse } from '../worker/jianpu.worker'

function enabledTracksForRender(
  parts: PartInfo[],
  disabledParts: ReadonlySet<string>,
): string[] | undefined {
  if (parts.length === 0) return undefined
  const enabled = parts
    .filter((part) => !disabledParts.has(part.abbreviation))
    .map((part) => part.abbreviation)
  if (enabled.length === parts.length) return undefined
  return enabled
}

function disabledLyricsForRender(
  parts: PartInfo[],
  disabledLyrics: ReadonlySet<string>,
): string[] | undefined {
  const lyricParts = parts.filter((part) => part.has_lyrics)
  if (lyricParts.length === 0) return undefined
  const disabled = lyricParts
    .filter((part) => disabledLyrics.has(part.abbreviation))
    .map((part) => part.abbreviation)
  if (disabled.length === 0) return undefined
  return disabled
}

interface JianpuWorkerState {
  parts: PartInfo[]
  partsLoading: boolean
  svgs: string[]
  wavUrl: string | null
  audioAvailable: boolean
  pdfAvailable: boolean
  pdfExporting: boolean
  splitPdfExporting: boolean
  diagnostics: Diagnostic[]
  rendering: boolean
  audioGenerating: boolean
  exportPdf: () => void
  exportSplitPdf: () => void
}

function downloadPdf(bytes: ArrayBuffer, filename: string) {
  const url = URL.createObjectURL(
    new Blob([bytes], { type: 'application/pdf' }),
  )
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = filename
  anchor.click()
  URL.revokeObjectURL(url)
}

function pdfFilenameFromActiveFile(activeFile: string): string {
  if (activeFile.endsWith('.jianpu')) {
    return activeFile.replace(/\.jianpu$/, '.pdf')
  }
  return `${activeFile}.pdf`
}

function zipFilenameFromActiveFile(activeFile: string): string {
  if (activeFile.endsWith('.jianpu')) {
    return activeFile.replace(/\.jianpu$/, '.zip')
  }
  return `${activeFile}.zip`
}

function baseNameFromActiveFile(activeFile: string): string {
  if (activeFile.endsWith('.jianpu')) {
    return activeFile.replace(/\.jianpu$/, '')
  }
  return activeFile
}

function downloadZip(bytes: ArrayBuffer, filename: string) {
  const url = URL.createObjectURL(
    new Blob([bytes], { type: 'application/zip' }),
  )
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = filename
  anchor.click()
  URL.revokeObjectURL(url)
}

export function useJianpuWorker(
  source: string,
  disabledParts: ReadonlySet<string>,
  disabledLyrics: ReadonlySet<string>,
  activeFile: string,
  debounceMs = 300,
): JianpuWorkerState {
  const [parts, setParts] = useState<PartInfo[]>([])
  const [partsLoading, setPartsLoading] = useState(false)
  const [svgs, setSvgs] = useState<string[]>([])
  const [wavUrl, setWavUrl] = useState<string | null>(null)
  const [audioAvailable, setAudioAvailable] = useState(false)
  const [pdfAvailable, setPdfAvailable] = useState(false)
  const [pdfExporting, setPdfExporting] = useState(false)
  const [splitPdfExporting, setSplitPdfExporting] = useState(false)
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [rendering, setRendering] = useState(false)
  const [audioGenerating, setAudioGenerating] = useState(false)

  const workerRef = useRef<Worker | null>(null)
  const wavUrlRef = useRef<string | null>(null)
  const partsRequestIdRef = useRef(0)
  const renderRequestIdRef = useRef(0)
  const pdfRequestIdRef = useRef(0)
  const splitPdfRequestIdRef = useRef(0)
  const latestPartsIdRef = useRef(0)
  const latestRenderIdRef = useRef(0)
  const latestPdfIdRef = useRef(0)
  const latestSplitPdfIdRef = useRef(0)
  const sourceRef = useRef(source)
  const activeFileRef = useRef(activeFile)
  const enabledTracksRef = useRef<string[] | undefined>(undefined)
  const disabledLyricsRef = useRef<string[] | undefined>(undefined)
  const audioAvailableRef = useRef(false)

  const enabledTracks = useMemo(
    () => enabledTracksForRender(parts, disabledParts),
    [parts, disabledParts],
  )
  const disabledLyricsTracks = useMemo(
    () => disabledLyricsForRender(parts, disabledLyrics),
    [parts, disabledLyrics],
  )

  sourceRef.current = source
  activeFileRef.current = activeFile
  enabledTracksRef.current = enabledTracks
  disabledLyricsRef.current = disabledLyricsTracks

  const setNextWavUrl = useCallback((next: string | null) => {
    if (wavUrlRef.current) {
      URL.revokeObjectURL(wavUrlRef.current)
    }
    wavUrlRef.current = next
    setWavUrl(next)
  }, [])

  useEffect(() => {
    const worker = new Worker(
      new URL('../worker/jianpu.worker.ts', import.meta.url),
      { type: 'module' },
    )
    workerRef.current = worker

    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const msg = event.data
      if (msg.type === 'ready') {
        audioAvailableRef.current = msg.audioAvailable
        setAudioAvailable(msg.audioAvailable)
        setPdfAvailable(msg.pdfAvailable)
        return
      }

      if (msg.type === 'parts') {
        if (msg.id !== latestPartsIdRef.current) return
        setPartsLoading(false)
        setParts(msg.parts)
        return
      }

      if (msg.type === 'pdf') {
        if (msg.id !== latestPdfIdRef.current) return
        setPdfExporting(false)
        downloadPdf(msg.pdf, pdfFilenameFromActiveFile(activeFileRef.current))
        return
      }

      if (msg.type === 'pdfErr') {
        if (msg.id !== latestPdfIdRef.current) return
        setPdfExporting(false)
        setDiagnostics(msg.diagnostics)
        return
      }

      if (msg.type === 'splitPdf') {
        if (msg.id !== latestSplitPdfIdRef.current) return
        setSplitPdfExporting(false)
        downloadZip(msg.zip, zipFilenameFromActiveFile(activeFileRef.current))
        return
      }

      if (msg.type === 'splitPdfErr') {
        if (msg.id !== latestSplitPdfIdRef.current) return
        setSplitPdfExporting(false)
        setDiagnostics(msg.diagnostics)
        return
      }

      if (msg.type === 'ok') {
        if (msg.id !== latestRenderIdRef.current) return
        setRendering(false)
        setSvgs(msg.svgs)
        setDiagnostics([])
        return
      }

      if (msg.type === 'audio') {
        if (msg.id !== latestRenderIdRef.current) return
        setAudioGenerating(false)
        setNextWavUrl(
          URL.createObjectURL(new Blob([msg.wav], { type: 'audio/wav' })),
        )
        return
      }

      if (msg.type === 'audioErr') {
        if (msg.id !== latestRenderIdRef.current) return
        setAudioGenerating(false)
        return
      }

      if (msg.type === 'err') {
        if (msg.id !== latestRenderIdRef.current) return
        setRendering(false)
        setAudioGenerating(false)
        setSvgs([])
        setNextWavUrl(null)
        setDiagnostics(msg.diagnostics)
      }
    }

    return () => {
      worker.terminate()
      workerRef.current = null
      if (wavUrlRef.current) {
        URL.revokeObjectURL(wavUrlRef.current)
        wavUrlRef.current = null
      }
    }
  }, [setNextWavUrl])

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

  useEffect(() => {
    const worker = workerRef.current
    if (!worker) return

    const id = ++renderRequestIdRef.current
    latestRenderIdRef.current = id
    setRendering(true)
    if (audioAvailableRef.current) {
      setAudioGenerating(true)
    }

    const timer = window.setTimeout(() => {
      const payload: WorkerRequest = {
        type: 'render',
        source,
        id,
        enabledTracks,
        disabledLyrics: disabledLyricsTracks,
      }
      worker.postMessage(payload)
    }, debounceMs)

    return () => window.clearTimeout(timer)
  }, [source, enabledTracks, disabledLyricsTracks, debounceMs])

  const exportPdf = useCallback(() => {
    const worker = workerRef.current
    if (!worker || pdfExporting || splitPdfExporting) return

    const id = ++pdfRequestIdRef.current
    latestPdfIdRef.current = id
    setPdfExporting(true)

    const payload: WorkerRequest = {
      type: 'generatePdf',
      source: sourceRef.current,
      id,
      enabledTracks: enabledTracksRef.current,
      disabledLyrics: disabledLyricsRef.current,
    }
    worker.postMessage(payload)
  }, [pdfExporting, splitPdfExporting])

  const exportSplitPdf = useCallback(() => {
    const worker = workerRef.current
    if (!worker || pdfExporting || splitPdfExporting) return

    const id = ++splitPdfRequestIdRef.current
    latestSplitPdfIdRef.current = id
    setSplitPdfExporting(true)

    const payload: WorkerRequest = {
      type: 'generateSplitPdf',
      source: sourceRef.current,
      id,
      baseName: baseNameFromActiveFile(activeFileRef.current),
    }
    worker.postMessage(payload)
  }, [pdfExporting, splitPdfExporting])

  return {
    parts,
    partsLoading,
    svgs,
    wavUrl,
    audioAvailable,
    pdfAvailable,
    pdfExporting,
    splitPdfExporting,
    diagnostics,
    rendering,
    audioGenerating,
    exportPdf,
    exportSplitPdf,
  }
}
