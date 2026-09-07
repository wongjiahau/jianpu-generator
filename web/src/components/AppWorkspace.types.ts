import type { RefObject } from 'react'
import type { NoteTimingOut, SvgDocumentOut } from '../jianpuWasm'
import type {
  Diagnostic,
  DiagnosticViewZone,
  EditorHandle,
  EditorSelection,
  LyricSpan,
  MeasureSpan,
  NoteSpan,
  PartDeclaration,
  PartInfo,
  PartMode,
  SoundfontValue,
} from '../types'
import type {
  MetadataFieldKey,
  ParsedMetadataFields,
} from '../utils/metadataSource'
import type { LyricCell, NoteCell } from './Preview'

export interface MeasureRange {
  start: number
  end: number
  /** Which measure the preview should scroll to for this selection, when
   * it differs from `start` — see `Preview`'s matching prop doc comment. */
  revealMeasureIndex: number
  /** The exact disjoint measure ranges to highlight in the SVG preview —
   * see `Preview`'s matching prop doc comment. */
  highlightRanges?: { start: number; end: number }[]
}

export interface AppWorkspaceProps {
  editorCollapsed: boolean
  setEditorCollapsed: (updater: (collapsed: boolean) => boolean) => void
  /** True while viewing a `#share=` or `#synced=` read-only preview — hides
   * the `Editor` entirely and the pane-divider toggle, since there is
   * nothing to edit or expand back into. */
  hideEditor: boolean
  editorRef: RefObject<EditorHandle | null>
  fileId: string
  source: string
  handleSourceChange: (value: string) => void
  /** "Format" toolbar action: drops redundant `# score` lines and
   * normalizes whitespace. */
  handleFormatScore: () => void
  /** "Octave up"/"Octave down" toolbar actions: shifts every note whose span
   * overlaps any of `ranges` — the editor's current selection, which a
   * multicursor selection (e.g. a clicked part label) surfaces as a
   * disjoint set rather than one span — by `delta` octaves (see
   * `source_edit::shift_range_octave`). */
  handleShiftSelectionOctave: (ranges: EditorSelection[], delta: number) => void
  readOnly: boolean
  diagnostics: Diagnostic[]
  diagnosticViewZones: DiagnosticViewZone[]
  measureSpans: MeasureSpan[]
  setSelectedLineRange: (
    range: { firstLine: number; lastLine: number } | null,
  ) => void
  notifySelection: (
    startLine: number,
    endLine: number,
    isEmpty: boolean,
    revealLine?: number,
    measureRanges?: { start: number; end: number }[],
  ) => void
  setEditPartsOpen: (open: boolean) => void
  setEditMetadataOpen: (open: boolean) => void
  forceSave: () => void
  measureAudioPlaying: boolean
  stopMeasurePlayback: () => void
  selectedMeasureRange: MeasureRange | null
  measureAudioGenerating: boolean
  soundfontReady: boolean
  playSelectedMeasures: () => void
  /** True while a note drag-select (see `useNoteSelection`) is active; when
   * set, the editor's Cmd/Ctrl+Enter shortcut plays the selected notes
   * instead of the measure(s) under the cursor. */
  notePlaybackSelectionActive: boolean
  playNoteSelection: () => void
  editPartsOpen: boolean
  partDeclarations: PartDeclaration[]
  parts: PartInfo[]
  handlePartDeclarationChange: (
    abbreviation: string,
    mode: PartMode,
    followTarget: string | null,
    soundfont: SoundfontValue | null,
    volume: number | null,
    octaveOffset: number | null,
  ) => void
  handleShiftPartOctave: (abbreviation: string, delta: number) => void
  previewInstrument: (programNumber: number) => void
  previewPercussion: (key: number) => void
  stopPreviewInstrument: () => void
  previewAudioPlaying: boolean
  editMetadataOpen: boolean
  parsedMetadata: ParsedMetadataFields
  handleMetadataFieldChange: (
    key: MetadataFieldKey,
    value: string | null,
  ) => void
  documents: SvgDocumentOut[]
  highlightedDocuments: SvgDocumentOut[]
  rendering: boolean
  handleSectionJump: (label: string) => void
  handleNoteRangeSelect: (selectedCells: NoteCell[]) => void
  /** Keeps the preview's note highlight in sync with the editor's own
   * current selection (see `useNoteSelection`'s
   * `handleEditorSelectionChange`), the reverse direction of
   * `handleNoteRangeSelect`. */
  handleEditorSelectionChange: (ranges: EditorSelection[]) => void
  selectedNoteCells: NoteCell[]
  /** Per-note/rest `(source_part_index, note_id) → measure_index` mapping,
   * used to resolve a measure click/drag into every note cell it contains
   * (see `Preview.tsx`'s `noteCellsInMeasureRange`) without relying on
   * pixel geometry. */
  noteSpans: NoteSpan[]
  /** Fired on mouseup after a lyric-syllable drag-select (see
   * `useLyricSelection`). Independent of `handleNoteRangeSelect` — a lyric
   * drag never selects/highlights notes and vice versa. */
  handleLyricRangeSelect: (selectedCells: LyricCell[]) => void
  /** Keeps the preview's lyric highlight in sync with the editor's own
   * current selection, the reverse direction of `handleLyricRangeSelect`
   * (mirrors `handleEditorSelectionChange`, kept fully separate). */
  handleLyricEditorSelectionChange: (ranges: EditorSelection[]) => void
  selectedLyricCells: LyricCell[]
  /** Per-lyric-syllable `(source_part_index, note_id, verse) → measure_index`
   * mapping, used to resolve a measure click/drag into every lyric cell it
   * contains alongside `noteSpans` (see `Preview.tsx`'s
   * `lyricCellsInMeasureRange`). */
  lyricSpans: LyricSpan[]
  /** Fired for a measure/bar-line click or drag with both the note cells and
   * lyric cells it resolved — see `Preview.tsx`'s `onMeasureRangeSelect`. */
  handleMeasureRangeSelect: (
    noteCells: NoteCell[],
    lyricCells: LyricCell[],
  ) => void
  audioGenerating: boolean
  wavUrl: string | null
  wavFilename: string
  mp3Exporting: boolean
  mp3Url: string | null
  mp3Filename: string
  onRequestAudioDownload: (url: string, filename: string) => void
  noteTimings: NoteTimingOut[]
  measureAudioNoteTimings: NoteTimingOut[]
  measureAudioElement: HTMLAudioElement | null
  noPartsSelected: boolean
}
