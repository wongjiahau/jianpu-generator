import MonacoEditor, { type Monaco, type OnMount } from '@monaco-editor/react'
import type { editor } from 'monaco-editor'
import {
  forwardRef,
  type ReactNode,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
} from 'react'
import type { Diagnostic, EditorHandle } from '../types'
import { byteOffsetToStringIndex } from '../utils/byteSpan'

export interface EditorProps {
  value: string
  onChange: (value: string) => void
  readOnly?: boolean
  diagnostics?: Diagnostic[]
  toolbar?: ReactNode
}

const MARKER_OWNER = 'jianpu'

function diagnosticRange(
  model: editor.ITextModel,
  source: string,
  diagnostic: Diagnostic,
  monacoApi: Monaco,
) {
  const startIndex = byteOffsetToStringIndex(source, diagnostic.span.start)
  const endIndex = Math.max(
    startIndex + 1,
    byteOffsetToStringIndex(source, diagnostic.span.end),
  )
  const startPos = model.getPositionAt(startIndex)
  const endPos = model.getPositionAt(endIndex)
  return new monacoApi.Range(
    startPos.lineNumber,
    startPos.column,
    endPos.lineNumber,
    endPos.column,
  )
}

export const Editor = forwardRef<EditorHandle, EditorProps>(function Editor(
  { value, onChange, readOnly = false, diagnostics = [], toolbar },
  ref,
) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null)
  const monacoRef = useRef<Monaco | null>(null)

  const applyDiagnostics = useCallback(() => {
    const ed = editorRef.current
    const monacoApi = monacoRef.current
    const model = ed?.getModel()
    if (!ed || !monacoApi || !model) return

    const source = model.getValue()

    if (diagnostics.length === 0) {
      monacoApi.editor.setModelMarkers(model, MARKER_OWNER, [])
      return
    }

    const markers = diagnostics.map((d) => {
      const range = diagnosticRange(model, source, d, monacoApi)
      return {
        severity:
          d.severity === 'warning'
            ? monacoApi.MarkerSeverity.Warning
            : monacoApi.MarkerSeverity.Error,
        message: d.message,
        startLineNumber: range.startLineNumber,
        startColumn: range.startColumn,
        endLineNumber: range.endLineNumber,
        endColumn: range.endColumn,
      }
    })

    monacoApi.editor.setModelMarkers(model, MARKER_OWNER, markers)
  }, [diagnostics])

  useImperativeHandle(ref, () => ({
    insertAtCursor(text: string) {
      const ed = editorRef.current
      const model = ed?.getModel()
      if (!ed || !model) return

      const selection = ed.getSelection()
      if (!selection) return

      ed.executeEdits('insertAtCursor', [
        {
          range: selection,
          text,
          forceMoveMarkers: true,
        },
      ])
      ed.focus()
    },
    getSelection() {
      const ed = editorRef.current
      const model = ed?.getModel()
      const selection = ed?.getSelection()
      if (!model || !selection) return { start: 0, end: 0 }

      return {
        start: model.getOffsetAt(selection.getStartPosition()),
        end: model.getOffsetAt(selection.getEndPosition()),
      }
    },
    setSelection(start: number, end: number) {
      const ed = editorRef.current
      const model = ed?.getModel()
      const monacoApi = monacoRef.current
      if (!ed || !model || !monacoApi) return

      const startPos = model.getPositionAt(start)
      const endPos = model.getPositionAt(end)
      ed.setSelection(
        new monacoApi.Selection(
          startPos.lineNumber,
          startPos.column,
          endPos.lineNumber,
          endPos.column,
        ),
      )
      ed.focus()
    },
    focus() {
      editorRef.current?.focus()
    },
    getEditor() {
      return editorRef.current
    },
  }))

  const handleMount: OnMount = (ed, monacoApi) => {
    editorRef.current = ed
    monacoRef.current = monacoApi
    applyDiagnostics()
  }

  useEffect(() => {
    applyDiagnostics()
  }, [applyDiagnostics])

  return (
    <div className="editor">
      {toolbar ? <div className="editor-toolbar">{toolbar}</div> : null}
      <div className="editor-surface">
        <MonacoEditor
          height="100%"
          language="plaintext"
          theme="vs"
          value={value}
          onChange={(next) => onChange(next ?? '')}
          onMount={handleMount}
          options={{
            readOnly,
            minimap: { enabled: false },
            fontFamily: 'var(--mono)',
            fontSize: 14,
            lineHeight: 21,
            padding: { top: 16, bottom: 16 },
            scrollBeyondLastLine: false,
            wordWrap: 'off',
            tabSize: 2,
            renderLineHighlight: 'none',
            renderValidationDecorations: 'on',
            overviewRulerLanes: 2,
            hideCursorInOverviewRuler: true,
            overviewRulerBorder: false,
            glyphMargin: false,
            folding: false,
            lineNumbers: 'on',
            lineNumbersMinChars: 3,
            scrollbar: {
              verticalScrollbarSize: 10,
              horizontalScrollbarSize: 10,
            },
          }}
        />
      </div>
    </div>
  )
})
