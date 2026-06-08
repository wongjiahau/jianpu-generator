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

interface JianpuWorkerState {
  parts: PartInfo[]
  partsLoading: boolean
  svgs: string[]
  wavUrl: string | null
  audioAvailable: boolean
  pdfAvailable: boolean
  pdfExporting: boolean
  diagnostics: Diagnostic[]
  rendering: boolean
  exportPdf: () => void
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

export function useJianpuWorker(
  source: string,
  disabledParts: ReadonlySet<string>,
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
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [rendering, setRendering] = useState(false)

  const workerRef = useRef<Worker | null>(null)
  const wavUrlRef = useRef<string | null>(null)
  const partsRequestIdRef = useRef(0)
  const renderRequestIdRef = useRef(0)
  const pdfRequestIdRef = useRef(0)
  const latestPartsIdRef = useRef(0)
  const latestRenderIdRef = useRef(0)
  const latestPdfIdRef = useRef(0)
  const sourceRef = useRef(source)
  const activeFileRef = useRef(activeFile)
  const enabledTracksRef = useRef<string[] | undefined>(undefined)

  const enabledTracks = useMemo(
    () => enabledTracksForRender(parts, disabledParts),
    [parts, disabledParts],
  )

  sourceRef.current = source
  activeFileRef.current = activeFile
  enabledTracksRef.current = enabledTracks

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

      if (msg.id !== latestRenderIdRef.current) return

      setRendering(false)
      if (msg.type === 'ok') {
        setSvgs(msg.svgs)
        setDiagnostics([])
        if (msg.wav) {
          setNextWavUrl(
            URL.createObjectURL(new Blob([msg.wav], { type: 'audio/wav' })),
          )
        } else {
          setNextWavUrl(null)
        }
      } else {
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

    const timer = window.setTimeout(() => {
      const payload: WorkerRequest = {
        type: 'render',
        source,
        id,
        enabledTracks,
      }
      worker.postMessage(payload)
    }, debounceMs)

    return () => window.clearTimeout(timer)
  }, [source, enabledTracks, debounceMs])

  const exportPdf = useCallback(() => {
    const worker = workerRef.current
    if (!worker || pdfExporting) return

    const id = ++pdfRequestIdRef.current
    latestPdfIdRef.current = id
    setPdfExporting(true)

    const payload: WorkerRequest = {
      type: 'generatePdf',
      source: sourceRef.current,
      id,
      enabledTracks: enabledTracksRef.current,
    }
    worker.postMessage(payload)
  }, [pdfExporting])

  return {
    parts,
    partsLoading,
    svgs,
    wavUrl,
    audioAvailable,
    pdfAvailable,
    pdfExporting,
    diagnostics,
    rendering,
    exportPdf,
  }
}
