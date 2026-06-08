type DiagnosticSeverity = 'error' | 'warning'

/** UTF-8 byte offsets into the source string. */
interface ByteSpan {
  start: number
  end: number
}

export interface Diagnostic {
  severity: DiagnosticSeverity
  message: string
  span: ByteSpan
  report?: string
}

type RenderOk = { status: 'ok'; svgs: string[] }

type RenderErr = { status: 'err'; diagnostics: Diagnostic[] }

export type RenderResult = RenderOk | RenderErr

export interface PartInfo {
  abbreviation: string
  display_name: string
}

type ListPartsOk = { status: 'ok'; parts: PartInfo[] }

type ListPartsErr = { status: 'err'; diagnostics: Diagnostic[] }

export type ListPartsResult = ListPartsOk | ListPartsErr

type GenerateWavOk = { status: 'ok'; wav: Uint8Array | number[] }

type GenerateWavErr = { status: 'err'; diagnostics: Diagnostic[] }

export type GenerateWavResult = GenerateWavOk | GenerateWavErr

type GeneratePdfOk = { status: 'ok'; pdf: Uint8Array | number[] }

type GeneratePdfErr = { status: 'err'; diagnostics: Diagnostic[] }

export type GeneratePdfResult = GeneratePdfOk | GeneratePdfErr

interface EditorSelection {
  start: number
  end: number
}

export interface EditorHandle {
  /** Insert text at the current cursor, replacing any selection. */
  insertAtCursor: (text: string) => void
  getSelection: () => EditorSelection
  setSelection: (start: number, end: number) => void
  focus: () => void
  getEditor: () => import('monaco-editor').editor.IStandaloneCodeEditor | null
}
