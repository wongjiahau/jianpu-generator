import init, { list_parts, render } from 'jianpu-wasm'
import type {
  Diagnostic,
  ListPartsResult,
  PartInfo,
  RenderResult,
} from '../types'

export type WorkerRequest =
  | { type: 'render'; source: string; id: number; enabledTracks?: string[] }
  | { type: 'listParts'; source: string; id: number }

export type WorkerResponse =
  | { type: 'ready' }
  | { type: 'ok'; id: number; svgs: string[] }
  | { type: 'err'; id: number; diagnostics: Diagnostic[] }
  | { type: 'parts'; id: number; parts: PartInfo[] }

let initialized = false

async function ensureInit() {
  if (!initialized) {
    await init()
    initialized = true
    postMessage({ type: 'ready' } satisfies WorkerResponse)
  }
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const msg = event.data
  await ensureInit()

  if (msg.type === 'listParts') {
    const result = list_parts(msg.source) as ListPartsResult
    if (result.status === 'ok') {
      postMessage({
        type: 'parts',
        id: msg.id,
        parts: result.parts,
      } satisfies WorkerResponse)
      return
    }

    postMessage({
      type: 'parts',
      id: msg.id,
      parts: [],
    } satisfies WorkerResponse)
    return
  }

  if (msg.type !== 'render') return

  const result = render(msg.source, msg.enabledTracks) as RenderResult
  if (result.status === 'ok') {
    postMessage({
      type: 'ok',
      id: msg.id,
      svgs: result.svgs,
    } satisfies WorkerResponse)
    return
  }

  postMessage({
    type: 'err',
    id: msg.id,
    diagnostics: result.diagnostics,
  } satisfies WorkerResponse)
}
