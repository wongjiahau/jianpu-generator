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
