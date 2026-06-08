import { useEffect, useMemo, useRef, useState } from 'react'
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
  diagnostics: Diagnostic[]
  rendering: boolean
}

export function useJianpuWorker(
  source: string,
  disabledParts: ReadonlySet<string>,
  debounceMs = 300,
): JianpuWorkerState {
  const [parts, setParts] = useState<PartInfo[]>([])
  const [partsLoading, setPartsLoading] = useState(false)
  const [svgs, setSvgs] = useState<string[]>([])
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [rendering, setRendering] = useState(false)

  const workerRef = useRef<Worker | null>(null)
  const partsRequestIdRef = useRef(0)
  const renderRequestIdRef = useRef(0)
  const latestPartsIdRef = useRef(0)
  const latestRenderIdRef = useRef(0)

  const enabledTracks = useMemo(
    () => enabledTracksForRender(parts, disabledParts),
    [parts, disabledParts],
  )

  useEffect(() => {
    const worker = new Worker(
      new URL('../worker/jianpu.worker.ts', import.meta.url),
      { type: 'module' },
    )
    workerRef.current = worker

    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const msg = event.data
      if (msg.type === 'ready') return

      if (msg.type === 'parts') {
        if (msg.id !== latestPartsIdRef.current) return
        setPartsLoading(false)
        setParts(msg.parts)
        return
      }

      if (msg.id !== latestRenderIdRef.current) return

      setRendering(false)
      if (msg.type === 'ok') {
        setSvgs(msg.svgs)
        setDiagnostics([])
      } else {
        setSvgs([])
        setDiagnostics(msg.diagnostics)
      }
    }

    return () => {
      worker.terminate()
      workerRef.current = null
    }
  }, [])

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

  return { parts, partsLoading, svgs, diagnostics, rendering }
}
