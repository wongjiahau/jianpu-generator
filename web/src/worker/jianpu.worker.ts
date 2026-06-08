import init, * as jianpuWasm from 'jianpu-wasm'
import { list_parts, render } from 'jianpu-wasm'
import type {
  Diagnostic,
  GeneratePdfResult,
  GenerateWavResult,
  ListPartsResult,
  PartInfo,
  RenderResult,
} from '../types'

const generateWav =
  'generate_wav' in jianpuWasm
    ? (jianpuWasm.generate_wav as (
        source: string,
        enabledTracks?: string[],
      ) => GenerateWavResult)
    : null

const generatePdf =
  'generate_pdf' in jianpuWasm
    ? (jianpuWasm.generate_pdf as (
        source: string,
        enabledTracks?: string[],
        disabledLyrics?: string[],
      ) => GeneratePdfResult)
    : null

export type WorkerRequest =
  | {
      type: 'render'
      source: string
      id: number
      enabledTracks?: string[]
      disabledLyrics?: string[]
    }
  | { type: 'listParts'; source: string; id: number }
  | {
      type: 'generatePdf'
      source: string
      id: number
      enabledTracks?: string[]
      disabledLyrics?: string[]
    }

export type WorkerResponse =
  | { type: 'ready'; audioAvailable: boolean; pdfAvailable: boolean }
  | { type: 'ok'; id: number; svgs: string[]; wav?: ArrayBuffer }
  | { type: 'err'; id: number; diagnostics: Diagnostic[] }
  | { type: 'parts'; id: number; parts: PartInfo[] }
  | { type: 'pdf'; id: number; pdf: ArrayBuffer }
  | { type: 'pdfErr'; id: number; diagnostics: Diagnostic[] }

let initialized = false

async function ensureInit() {
  if (!initialized) {
    await init()
    initialized = true
    postMessage({
      type: 'ready',
      audioAvailable: generateWav !== null,
      pdfAvailable: generatePdf !== null,
    } satisfies WorkerResponse)
  }
}

function binaryBufferFromResult(bytes: Uint8Array | number[]): ArrayBuffer {
  const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
  if (view.byteOffset === 0 && view.byteLength === view.buffer.byteLength) {
    return view.buffer as ArrayBuffer
  }
  return view.buffer.slice(
    view.byteOffset,
    view.byteOffset + view.byteLength,
  ) as ArrayBuffer
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

  if (msg.type === 'generatePdf') {
    if (!generatePdf) {
      postMessage({
        type: 'pdfErr',
        id: msg.id,
        diagnostics: [
          {
            severity: 'error',
            message: 'PDF export is not available in this build.',
            span: { start: 0, end: 0 },
          },
        ],
      } satisfies WorkerResponse)
      return
    }

    const result = generatePdf(
      msg.source,
      msg.enabledTracks,
      msg.disabledLyrics,
    )
    if (result.status === 'ok') {
      const pdfBuffer = binaryBufferFromResult(result.pdf)
      postMessage(
        {
          type: 'pdf',
          id: msg.id,
          pdf: pdfBuffer,
        } satisfies WorkerResponse,
        { transfer: [pdfBuffer] },
      )
      return
    }

    postMessage({
      type: 'pdfErr',
      id: msg.id,
      diagnostics: result.diagnostics,
    } satisfies WorkerResponse)
    return
  }

  if (msg.type !== 'render') return

  const result = render(
    msg.source,
    msg.enabledTracks,
    msg.disabledLyrics,
  ) as RenderResult
  if (result.status === 'ok') {
    let wavBuffer: ArrayBuffer | undefined
    if (generateWav) {
      const wavResult = generateWav(msg.source, msg.enabledTracks)
      if (wavResult.status === 'ok') {
        wavBuffer = binaryBufferFromResult(wavResult.wav)
      }
    }

    if (wavBuffer) {
      postMessage(
        {
          type: 'ok',
          id: msg.id,
          svgs: result.svgs,
          wav: wavBuffer,
        } satisfies WorkerResponse,
        { transfer: [wavBuffer] },
      )
      return
    }

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
