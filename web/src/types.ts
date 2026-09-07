export type {
  DiagnosticMessageOut as DiagnosticMessage,
  DiagnosticOut as Diagnostic,
  DiagnosticViewZoneOut as DiagnosticViewZone,
  GeneratePdfResponse as GeneratePdfResult,
  GenerateSplitPdfsResponse as GenerateSplitPdfResult,
  GenerateWavResponse as GenerateWavResult,
  ListLyricSpansResponse as ListLyricSpansResult,
  ListMeasureSpansResponse as ListMeasureSpansResult,
  ListNoteSpansResponse as ListNoteSpansResult,
  ListPartsResponse as ListPartsResult,
  LyricSpanOut as LyricSpan,
  MeasureAtOffsetResponse as MeasureAtOffsetResult,
  MeasureSpanOut as MeasureSpan,
  NoteSpanOut as NoteSpan,
  PartDeclarationModeOut as PartMode,
  PartDeclarationOut as PartDeclaration,
  PartOut as PartInfo,
  RenderResponse as RenderResult,
  SectionRangeOut as SectionRange,
  SequenceEntryOut as SequenceEntry,
  SpanOut as ByteSpan,
} from './jianpuWasm'

// Format: "N: Instrument Name" e.g. "48: String Ensemble 1"
export type SoundfontValue = string

export interface EditorSelection {
  start: number
  end: number
}

export interface EditorHandle {
  /** Insert text at the current cursor, replacing any selection. */
  insertAtCursor: (text: string) => void
  getSelection: () => EditorSelection
  setSelection: (start: number, end: number) => void
  /**
   * Select a disjoint set of ranges (Monaco multicursor) by UTF-8 byte
   * offset, matching `setSelection`'s convention. Reveals the first range.
   */
  setSelections: (ranges: Array<{ start: number; end: number }>) => void
  /**
   * Replace the entire model content with `newSource` and select `ranges`
   * (byte offsets into `newSource`) — both synchronously, in the same call,
   * so no other effect can observe the new text with a stale selection in
   * between. Used by the "shift selection octave" toolbar action: unlike
   * `onChange` + a later `setSelections` call, this closes the race where
   * `Editor.tsx`'s own generic post-edit selection restore (see its
   * `savedSelectionsRef` effect pair) would otherwise re-apply the
   * *pre-edit* selection's stale line/column positions over top of the
   * correct one (see `HANDOFF-octave-toolbar-part-label-selection-bug.md`).
   */
  replaceContentWithSelections: (
    newSource: string,
    ranges: Array<{ start: number; end: number }>,
  ) => void
  /** Select a range of lines by 1-indexed line numbers and reveal the start. */
  setSelectionByLines: (startLine: number, endLine: number) => void
  /**
   * Select a disjoint set of whole-line ranges (Monaco multicursor) by
   * 1-indexed line numbers — the line-range analogue of `setSelections`,
   * for callers (e.g. a `# sequence` chain selection spanning out-of-order
   * sections) that resolve to lines rather than byte offsets. Reveals
   * `revealStartLine` (defaulting to the first range's start) — two
   * disjoint ranges can sit too far apart in the source to both fit on
   * screen at once, so callers should pass whichever one the user is
   * currently pointing at.
   */
  setSelectionsByLines: (
    ranges: Array<{ startLine: number; endLine: number }>,
    revealStartLine?: number,
  ) => void
  /** Move the cursor to the given JS string char offset and reveal the line. */
  jumpToOffset: (charOffset: number) => void
  focus: () => void
  getEditor: () => import('monaco-editor').editor.IStandaloneCodeEditor | null
}
