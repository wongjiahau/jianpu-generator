import MonacoEditor, { type Monaco, type OnMount } from '@monaco-editor/react'
import type { editor, IDisposable, ISelection, languages } from 'monaco-editor'
import {
  forwardRef,
  type ReactNode,
  useCallback,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useRef,
} from 'react'
import {
  JIANPU_LANGUAGE_ID,
  registerJianpuLanguage,
} from '../monacoJianpuLanguage'
import { registerJianpuRenameProvider } from '../monacoRenameProvider'
import type {
  Diagnostic,
  DiagnosticViewZone,
  EditorHandle,
  EditorSelection,
  MeasureSpan,
} from '../types'
import { stringIndexToByteOffset } from '../utils/byteSpan'
import {
  buildDiagnosticMarkers,
  createDiagnosticViewZoneDomNode,
  errorViewZoneHeightInPx,
} from './editorDiagnosticViewZones'
import { createEditorImperativeHandle } from './editorImperativeHandle'
import {
  createMeasureViewZoneDomNode,
  measureViewZoneLineNumber,
} from './editorMeasureViewZones'
import { defineJianpuEditorTheme, EDITOR_THEME } from './editorTheme'

export interface EditorProps {
  /** Unique per-file ID; gives each file its own Monaco model and undo stack. */
  path?: string
  value: string
  onChange: (value: string) => void
  readOnly?: boolean
  diagnostics?: Diagnostic[]
  diagnosticViewZones?: DiagnosticViewZone[]
  measureSpans?: MeasureSpan[]
  toolbar?: ReactNode
  /** `isEmpty` is true when the selection is a plain caret (0-length) rather
   * than a highlighted range — callers use this to gate the preview's
   * measure-background highlight, which should only show for a bare caret. */
  onSelectionChange?: (
    startLine: number,
    endLine: number,
    isEmpty: boolean,
  ) => void
  /** Same selection as `onSelectionChange`, but as UTF-8 byte offsets — one
   * entry per selection in Monaco's current (possibly multicursor)
   * selection set, via `ed.getSelections()`/`stringIndexToByteOffset` — used
   * by callers whose measure mapping is byte-offset-based rather than
   * line-based (see `useNoteSelection`'s `handleEditorSelectionChange`) and
   * by the "shift selection octave" toolbar action, which needs every
   * disjoint piece of a multicursor selection (e.g. one produced by
   * clicking a part label), not just the primary one. `isEmpty` is true
   * only when every selection in the set is empty. */
  onSelectionOffsetChange?: (
    ranges: EditorSelection[],
    isEmpty: boolean,
  ) => void
  onCursorLineChange?: (line: number) => void
  onPlayMeasure?: () => void
  onForceSave?: () => void
  onEditPartsClick?: () => void
  onEditMetadataClick?: () => void
}

const MARKER_OWNER = 'jianpu'

export const Editor = forwardRef<EditorHandle, EditorProps>(function Editor(
  {
    path,
    value,
    onChange,
    readOnly = false,
    diagnostics = [],
    diagnosticViewZones = [],
    measureSpans = [],
    toolbar,
    onSelectionChange,
    onSelectionOffsetChange,
    onCursorLineChange,
    onPlayMeasure,
    onForceSave,
    onEditPartsClick,
    onEditMetadataClick,
  },
  ref,
) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null)
  const monacoRef = useRef<Monaco | null>(null)
  const measureViewZoneIdsRef = useRef<string[]>([])
  const diagnosticViewZoneIdsRef = useRef<string[]>([])
  const onSelectionChangeRef = useRef(onSelectionChange)
  const onSelectionOffsetChangeRef = useRef(onSelectionOffsetChange)
  const onCursorLineChangeRef = useRef(onCursorLineChange)
  const onPlayMeasureRef = useRef(onPlayMeasure)
  const onForceSaveRef = useRef(onForceSave)
  const onEditPartsClickRef = useRef(onEditPartsClick)
  const onEditMetadataClickRef = useRef(onEditMetadataClick)
  const savedSelectionsRef = useRef<ISelection[] | null>(null)
  const codeLensProviderRef = useRef<IDisposable | null>(null)
  useEffect(() => {
    onSelectionChangeRef.current = onSelectionChange
    onSelectionOffsetChangeRef.current = onSelectionOffsetChange
    onCursorLineChangeRef.current = onCursorLineChange
    onPlayMeasureRef.current = onPlayMeasure
    onForceSaveRef.current = onForceSave
    onEditPartsClickRef.current = onEditPartsClick
    onEditMetadataClickRef.current = onEditMetadataClick
  })

  const applyDiagnostics = useCallback(() => {
    const ed = editorRef.current
    const monacoApi = monacoRef.current
    const model = ed?.getModel()
    if (!ed || !monacoApi || !model) return

    const markers = buildDiagnosticMarkers(model, monacoApi, diagnostics)
    monacoApi.editor.setModelMarkers(model, MARKER_OWNER, markers)
  }, [diagnostics])

  const applyMeasureViewZones = useCallback(() => {
    const ed = editorRef.current
    const model = ed?.getModel()
    if (!ed || !model) return

    ed.changeViewZones((accessor) => {
      for (const id of measureViewZoneIdsRef.current) {
        accessor.removeZone(id)
      }
      measureViewZoneIdsRef.current = []

      const source = model.getValue()

      measureSpans.forEach((span, index) => {
        const lineNumber = measureViewZoneLineNumber(model, source, span)
        const domNode = createMeasureViewZoneDomNode(span, index)

        const id = accessor.addZone({
          afterLineNumber: lineNumber - 1,
          heightInLines: 1,
          domNode,
        })
        measureViewZoneIdsRef.current.push(id)
      })
    })
  }, [measureSpans])

  const applyDiagnosticViewZones = useCallback(() => {
    const ed = editorRef.current
    const model = ed?.getModel()
    if (!ed || !model) return

    ed.changeViewZones((accessor) => {
      for (const id of diagnosticViewZoneIdsRef.current) {
        accessor.removeZone(id)
      }
      diagnosticViewZoneIdsRef.current = []

      for (const zone of diagnosticViewZones) {
        const domNode = createDiagnosticViewZoneDomNode(
          zone.severity,
          zone.messages,
        )
        const heightInPx = errorViewZoneHeightInPx(
          domNode,
          ed.getLayoutInfo().contentWidth,
        )
        const id = accessor.addZone({
          afterLineNumber: zone.after_line_number,
          heightInPx,
          domNode,
        })
        diagnosticViewZoneIdsRef.current.push(id)
      }
    })
  }, [diagnosticViewZones])

  useImperativeHandle(
    ref,
    () => createEditorImperativeHandle(editorRef, monacoRef),
    [],
  )

  useEffect(() => {
    return () => {
      codeLensProviderRef.current?.dispose()
    }
  }, [])

  const handleMount: OnMount = (ed, monacoApi) => {
    editorRef.current = ed
    monacoRef.current = monacoApi
    applyDiagnostics()
    applyMeasureViewZones()
    applyDiagnosticViewZones()

    ed.addCommand(monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.Enter, () =>
      onPlayMeasureRef.current?.(),
    )

    ed.addCommand(monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyS, () =>
      onForceSaveRef.current?.(),
    )

    const editPartsCommandId = ed.addCommand(0, () => {
      onEditPartsClickRef.current?.()
    })

    const editMetadataCommandId = ed.addCommand(0, () => {
      onEditMetadataClickRef.current?.()
    })

    codeLensProviderRef.current?.dispose()
    codeLensProviderRef.current = monacoApi.languages.registerCodeLensProvider(
      JIANPU_LANGUAGE_ID,
      {
        provideCodeLenses(model: editor.ITextModel) {
          const lenses: languages.CodeLens[] = []
          for (let line = 1; line <= model.getLineCount(); line++) {
            if (model.getLineContent(line).trim() === '# parts') {
              lenses.push({
                range: new monacoApi.Range(line, 1, line, 1),
                command: {
                  id: editPartsCommandId ?? '',
                  title: 'Edit Parts',
                },
              })
            }
            if (model.getLineContent(line).trim() === '# metadata') {
              lenses.push({
                range: new monacoApi.Range(line, 1, line, 1),
                command: {
                  id: editMetadataCommandId ?? '',
                  title: 'Edit Metadata',
                },
              })
            }
          }
          return { lenses, dispose: () => {} }
        },
      },
    )

    const notifyCursor = () => {
      const model = ed.getModel()
      if (!model) return
      const selection = ed.getSelection()
      if (!selection) return
      const isEmpty = selection.isEmpty()
      onSelectionChangeRef.current?.(
        selection.startLineNumber,
        selection.endLineNumber,
        isEmpty,
      )
      if (onSelectionOffsetChangeRef.current) {
        const text = model.getValue()
        // A multicursor selection (e.g. one produced by clicking a part
        // label, which selects that part's notes across every measure in
        // its system) surfaces as several disjoint selections here, not
        // one — getSelections()[0] is the primary/anchor selection,
        // matching `selection` above.
        const selections = ed.getSelections() ?? [selection]
        const ranges: EditorSelection[] = selections.map((sel) => ({
          start: stringIndexToByteOffset(
            text,
            model.getOffsetAt(sel.getStartPosition()),
          ),
          end: stringIndexToByteOffset(
            text,
            model.getOffsetAt(sel.getEndPosition()),
          ),
        }))
        const allEmpty = selections.every((sel) => sel.isEmpty())
        onSelectionOffsetChangeRef.current(ranges, allEmpty)
      }
      onCursorLineChangeRef.current?.(selection.startLineNumber)
    }
    ed.onDidChangeCursorPosition(notifyCursor)
    notifyCursor()
  }

  useEffect(() => {
    applyDiagnostics()
  }, [applyDiagnostics])

  useEffect(() => {
    applyMeasureViewZones()
  }, [applyMeasureViewZones])

  useEffect(() => {
    applyDiagnosticViewZones()
  }, [applyDiagnosticViewZones])

  // @monaco-editor/react calls model.executeEdits() (via useEffect) whenever
  // the value prop doesn't match the model's current text, which relocates
  // the cursor to the end of the document (forceMoveMarkers: true). The fix
  // has two parts:
  //
  // 1. useLayoutEffect runs BEFORE the child's useEffect, so we snapshot
  //    every selection here (a multicursor drag-select can push more than
  //    one) before executeEdits has a chance to move them.
  // 2. useEffect runs AFTER the child's useEffect (executeEdits + move), so
  //    we restore the snapshotted selections here.
  //
  // This runs unconditionally, even for edits that originated from the
  // user's own typing (echoed back down as this `value`): by the time this
  // effect pair runs, the model's text already matches `value` (Monaco
  // applied the keystroke synchronously before `onChange` fired), so
  // @monaco-editor/react's `value !== model.getValue()` check skips
  // executeEdits — but `setSelections` below still runs regardless, and is
  // NOT a no-op: it must actively re-apply every snapshotted selection, or
  // Monaco's own post-keystroke state (which already collapsed a
  // multicursor edit down to N empty carets, one per edited range) would be
  // left standing. An earlier version restored only the *primary* selection
  // here (`getSelection()`/`setSelection()`, both singular), which for a
  // single caret was an effective no-op but for a multicursor selection
  // silently discarded every cursor but the first on every keystroke. An
  // even earlier version gated the snapshot/restore behind a same-origin
  // flag, but the flag was a single ref not scoped to a particular `value`
  // transition — an external update (e.g. formatScore, part-declaration
  // edits) landing while the flag was still set from a recent keystroke
  // would skip the restore it actually needed, letting the cursor jump to
  // the end and stick.
  // biome-ignore lint/correctness/useExhaustiveDependencies: value is the trigger; refs don't need to be listed
  useLayoutEffect(() => {
    savedSelectionsRef.current = editorRef.current?.getSelections() ?? null
  }, [value])

  // biome-ignore lint/correctness/useExhaustiveDependencies: value is the trigger; refs don't need to be listed
  useEffect(() => {
    const ed = editorRef.current
    const saved = savedSelectionsRef.current
    if (ed && saved && saved.length > 0) {
      ed.setSelections(saved)
    }
  }, [value])

  return (
    <div className="editor">
      {toolbar ? <div className="editor-toolbar">{toolbar}</div> : null}
      <div className="editor-surface">
        <MonacoEditor
          height="100%"
          language={JIANPU_LANGUAGE_ID}
          theme={EDITOR_THEME}
          path={path}
          value={value}
          onChange={(next) => onChange(next ?? '')}
          beforeMount={(monacoApi) => {
            registerJianpuLanguage(monacoApi)
            registerJianpuRenameProvider(monacoApi)
            defineJianpuEditorTheme(monacoApi)
          }}
          onMount={handleMount}
          options={{
            readOnly,
            codeLens: true,
            minimap: { enabled: false },
            fontFamily: 'var(--mono)',
            fontSize: 14,
            lineHeight: 21,
            padding: { top: 16, bottom: 16 },
            scrollBeyondLastLine: false,
            wordWrap: 'off',
            tabSize: 2,
            renderLineHighlight: 'line',
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
