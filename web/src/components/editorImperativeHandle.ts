import type { Monaco } from '@monaco-editor/react'
import type { editor } from 'monaco-editor'
import type { RefObject } from 'react'
import type { EditorHandle } from '../types'
import { buildMonacoSelections } from '../utils/noteSpanSelection'

/** Builds the imperative `EditorHandle` exposed via `useImperativeHandle`, wiring each
 * method to the Monaco editor/API instances captured in refs by `onMount`. */
export function createEditorImperativeHandle(
  editorRef: RefObject<editor.IStandaloneCodeEditor | null>,
  monacoRef: RefObject<Monaco | null>,
): EditorHandle {
  return {
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
    setSelections(ranges: Array<{ start: number; end: number }>) {
      const ed = editorRef.current
      const model = ed?.getModel()
      const monacoApi = monacoRef.current
      if (!ed || !model || !monacoApi || ranges.length === 0) return

      const source = model.getValue()
      const selections = buildMonacoSelections(
        ranges.map((range) => ({
          startByte: range.start,
          endByte: range.end,
        })),
        source,
        monacoApi,
        model,
      )
      ed.setSelections(selections)
      ed.revealRangeInCenter(selections[0])
      ed.focus()
    },
    replaceContentWithSelections(
      newSource: string,
      ranges: Array<{ start: number; end: number }>,
    ) {
      const ed = editorRef.current
      const model = ed?.getModel()
      const monacoApi = monacoRef.current
      if (!ed || !model || !monacoApi) return

      // Step 1: apply the new text first — the byte→line/column mapping
      // `buildMonacoSelections` needs below only works once the model
      // already reflects `newSource`. Deliberately no `endCursorState` here;
      // see the doc comment on `EditorHandle.replaceContentWithSelections`.
      ed.executeEdits('replaceContentWithSelections', [
        {
          range: model.getFullModelRange(),
          text: newSource,
          forceMoveMarkers: true,
        },
      ])

      if (ranges.length === 0) return

      // Step 2: now that the model's layout matches `newSource`, resolve
      // the byte ranges to positions and select them — synchronously, in
      // the same call stack as step 1, so nothing else can observe the new
      // text with a stale selection in between.
      const selections = buildMonacoSelections(
        ranges.map((range) => ({
          startByte: range.start,
          endByte: range.end,
        })),
        newSource,
        monacoApi,
        model,
      )
      ed.setSelections(selections)
      ed.revealRangeInCenter(selections[0])
    },
    setSelectionByLines(startLine: number, endLine: number) {
      const ed = editorRef.current
      if (!ed) return
      ed.setSelection({
        startLineNumber: startLine,
        startColumn: 1,
        endLineNumber: endLine,
        endColumn: ed.getModel()?.getLineMaxColumn(endLine) ?? 1,
      })
      ed.revealLineInCenter(startLine)
    },
    setSelectionsByLines(ranges, revealStartLine) {
      const ed = editorRef.current
      const model = ed?.getModel()
      const monacoApi = monacoRef.current
      if (!ed || !model || !monacoApi || ranges.length === 0) return

      const selections = ranges.map(
        (range) =>
          new monacoApi.Selection(
            range.startLine,
            1,
            range.endLine,
            model.getLineMaxColumn(range.endLine),
          ),
      )
      ed.setSelections(selections)
      ed.revealLineInCenter(revealStartLine ?? ranges[0].startLine)
      ed.focus()
    },
    jumpToOffset(charOffset: number) {
      const ed = editorRef.current
      const model = ed?.getModel()
      if (!ed || !model) return
      const position = model.getPositionAt(charOffset)
      ed.setPosition(position)
      ed.revealPositionInCenter(position)
      ed.focus()
    },
    focus() {
      editorRef.current?.focus()
    },
    getEditor() {
      return editorRef.current
    },
  }
}
