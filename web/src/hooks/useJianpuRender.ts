import { useEffect, useRef, useState } from 'react'
import type { Diagnostic } from '../types'
import type { WorkerRequest, WorkerResponse } from '../worker/jianpu.worker'

interface RenderState {
  svgs: string[]
  diagnostics: Diagnostic[]
  rendering: boolean
}

export function useJianpuRender(source: string, debounceMs = 300): RenderState {
  const [svgs, setSvgs] = useState<string[]>([])
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])
  const [rendering, setRendering] = useState(false)

  const workerRef = useRef<Worker | null>(null)
  const requestIdRef = useRef(0)
  const latestIdRef = useRef(0)

  useEffect(() => {
    const worker = new Worker(
      new URL('../worker/jianpu.worker.ts', import.meta.url),
      { type: 'module' },
    )
    workerRef.current = worker

    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const msg = event.data
      if (msg.type === 'ready') return
      if (msg.id !== latestIdRef.current) return

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

    const id = ++requestIdRef.current
    latestIdRef.current = id
    setRendering(true)

    const timer = window.setTimeout(() => {
      const payload: WorkerRequest = { type: 'render', source, id }
      worker.postMessage(payload)
    }, debounceMs)

    return () => window.clearTimeout(timer)
  }, [source, debounceMs])

  return { svgs, diagnostics, rendering }
}
