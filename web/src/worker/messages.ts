import type { NoteTimingOut, SvgDocumentOut } from '../jianpuWasm'
import type {
  Diagnostic,
  DiagnosticViewZone,
  EditorSelection,
  LyricSpan,
  MeasureSpan,
  NoteSpan,
  PartDeclaration,
  PartInfo,
  PartMode,
  SectionRange,
  SequenceEntry,
} from '../types'
import type { NoteCell } from '../utils/noteSpanSelection'

export type WorkerRequest =
  | { type: 'wasmModule'; module: WebAssembly.Module }
  | {
      type: 'loadSoundfont'
      soundfont: ArrayBuffer
    }
  | {
      type: 'loadPdfFonts'
      scFont: ArrayBuffer
      tcFont: ArrayBuffer
      monoFont: ArrayBuffer
    }
  | {
      type: 'render'
      source: string
      id: number
      enabledTracks?: string[]
      disabledLyrics?: string[]
    }
  | { type: 'listParts'; source: string; id: number }
  | {
      type: 'updatePartDeclaration'
      source: string
      abbreviation: string
      mode: PartMode
      followTarget: string | null
      soundfont: string | null
      volume: number | null
      octaveOffset: number | null
      id: number
    }
  | {
      type: 'generatePdf'
      source: string
      id: number
      enabledTracks?: string[]
      disabledLyrics?: string[]
    }
  | {
      type: 'generateSplitPdf'
      source: string
      id: number
      baseName: string
    }
  | {
      type: 'generateMidi'
      source: string
      id: number
      enabledTracks?: string[]
    }
  | {
      type: 'generateSplitMidi'
      source: string
      id: number
      baseName: string
    }
  | {
      type: 'generateSplitWav'
      source: string
      id: number
      baseName: string
    }
  | {
      type: 'generateMp3'
      source: string
      id: number
      enabledTracks?: string[]
    }
  | {
      type: 'generateSplitMp3'
      source: string
      id: number
      baseName: string
    }
  | {
      type: 'generateAudio'
      source: string
      id: number
      enabledTracks?: string[]
    }
  | {
      type: 'generateMeasureRangeAudio'
      source: string
      id: number
      startMeasureIndex: number
      endMeasureIndex: number
      extendToLastOccurrence: boolean
      respectSequence: boolean
      /**
       * 0-based index range into `# sequence` (the order entries are
       * written in `# sequence`) naming the exact entry/entries selected,
       * so a repeated label (e.g. `A, B(-x), B`) resolves to the clicked
       * occurrence instead of always the first one sharing that written
       * measure range. Omit both when the range isn't a `# sequence`
       * selection (e.g. "play current measure").
       */
      sequenceEntryStartIndex?: number
      sequenceEntryEndIndex?: number
      enabledTracks?: string[]
      /**
       * The part-visibility toggle's current state — unlike `enabledTracks`
       * (which "play selection" overrides down to just the drag-selected
       * parts, muting this clip's audio), this is always the same set the
       * currently-rendered SVG was compacted against, never a playback-only
       * override. Passed straight through to WASM's own `visible_tracks`
       * parameter (see `list_note_timings_for_range`), so each returned
       * `NoteTiming`'s `source_part_index` already agrees with the SVG's
       * `data-part-index` — including block/note-id structure such as a
       * `MultiMeasureRest` run only created once a hidden sibling part's
       * notes are removed (see **Note identity** in `ARCHITECTURE.md`).
       * `undefined` means no part is hidden.
       */
      visibleTracks?: string[]
      /**
       * When present, narrows the generated clip down to exactly these
       * drag-selected `(sourcePartIndex, noteId)` cells' elapsed-seconds
       * span (sample-accurately trimmed/faded in Rust — see
       * `jianpu_generator::wav::TrimWindow`) instead of playing the whole
       * `[startMeasureIndex, endMeasureIndex]` range — what the web app's
       * "play selection" needs. Omit for every other measure-range playback
       * (e.g. "play current measure"/"play from current measure"/"play
       * all"), which always plays the range in full.
       */
      trimToSelectedNoteCells?: NoteCell[]
    }
  | {
      type: 'renderWithHighlightRange'
      source: string
      id: number
      /** Disjoint, inclusive measure-index ranges to highlight — a `#
       * sequence` chain selection spanning out-of-order entries can
       * highlight several disjoint measures at once (e.g. "C" and a later
       * repeat of "A", but not "B" in between). */
      ranges: { start: number; end: number }[]
      enabledTracks?: string[]
      disabledLyrics?: string[]
    }
  | { type: 'listMeasureSpans'; source: string; id: number }
  | {
      type: 'listNoteSpans'
      source: string
      id: number
      enabledTracks?: string[]
    }
  | {
      type: 'listLyricSpans'
      source: string
      id: number
      enabledTracks?: string[]
    }
  | { type: 'previewInstrument'; id: number; programNumber: number }
  | { type: 'previewPercussion'; id: number; key: number }
  | {
      type: 'importFromFile'
      id: number
      bytes: ArrayBuffer
      kind: 'svg' | 'pdf'
    }
  | {
      type: 'formatScore'
      source: string
      id: number
    }
  | {
      type: 'shiftPartOctave'
      source: string
      abbreviation: string
      delta: number
      id: number
    }
  | {
      type: 'shiftRangeOctave'
      source: string
      // A disjoint set of ranges, not one min/max span — a multicursor
      // selection (e.g. clicking a part label, which selects that part's
      // notes across every measure in its system) is generally disjoint;
      // collapsing it to one span would sweep in unrelated notes/parts
      // sitting between the disjoint pieces (see
      // `source_edit::shift_range_octave`'s doc comment).
      ranges: EditorSelection[]
      delta: number
      id: number
    }

export type WorkerResponse =
  | {
      type: 'ready'
      audioAvailable: boolean
      pdfAvailable: boolean
      midiAvailable: boolean
      mp3Available: boolean
    }
  | {
      type: 'ok'
      id: number
      documents: SvgDocumentOut[]
      diagnostics: Diagnostic[]
      diagnosticViewZones: DiagnosticViewZone[]
    }
  | {
      type: 'audio'
      id: number
      wav: ArrayBuffer
      noteTimings: NoteTimingOut[]
    }
  | { type: 'audioErr'; id: number }
  | {
      type: 'err'
      id: number
      diagnostics: Diagnostic[]
      diagnosticViewZones: DiagnosticViewZone[]
    }
  | {
      type: 'parts'
      id: number
      parts: PartInfo[]
      declarations: PartDeclaration[]
    }
  | {
      type: 'partDeclarationUpdated'
      id: number
      source: string
      declarations: PartDeclaration[]
    }
  | { type: 'pdf'; id: number; pdf: ArrayBuffer }
  | { type: 'pdfErr'; id: number; diagnostics: Diagnostic[] }
  | { type: 'splitPdf'; id: number; zip: ArrayBuffer }
  | { type: 'splitPdfErr'; id: number; diagnostics: Diagnostic[] }
  | { type: 'midi'; id: number; midi: ArrayBuffer }
  | { type: 'midiErr'; id: number; diagnostics: Diagnostic[] }
  | { type: 'splitMidi'; id: number; zip: ArrayBuffer }
  | { type: 'splitMidiErr'; id: number; diagnostics: Diagnostic[] }
  | { type: 'splitWav'; id: number; zip: ArrayBuffer }
  | { type: 'splitWavErr'; id: number; diagnostics: Diagnostic[] }
  | { type: 'mp3'; id: number; mp3: ArrayBuffer; noteTimings: NoteTimingOut[] }
  | { type: 'mp3Err'; id: number; diagnostics: Diagnostic[] }
  | { type: 'splitMp3'; id: number; zip: ArrayBuffer }
  | { type: 'splitMp3Err'; id: number; diagnostics: Diagnostic[] }
  | {
      type: 'measureRangeAudio'
      id: number
      wav: ArrayBuffer
      noteTimings: NoteTimingOut[]
    }
  | { type: 'measureRangeAudioErr'; id: number }
  | { type: 'instrumentPreview'; id: number; wav: ArrayBuffer }
  | { type: 'instrumentPreviewErr'; id: number }
  | { type: 'percussionPreview'; id: number; wav: ArrayBuffer }
  | { type: 'percussionPreviewErr'; id: number }
  | { type: 'highlightRangeOk'; id: number; documents: SvgDocumentOut[] }
  | { type: 'highlightRangeErr'; id: number; diagnostics: Diagnostic[] }
  | {
      type: 'measureSpans'
      id: number
      status: 'ok' | 'err'
      spans: MeasureSpan[]
      sectionRanges: SectionRange[]
      sequenceEntries: SequenceEntry[]
    }
  | {
      type: 'noteSpans'
      id: number
      status: 'ok' | 'err'
      spans: NoteSpan[]
    }
  | {
      type: 'lyricSpans'
      id: number
      status: 'ok' | 'err'
      spans: LyricSpan[]
    }
  | { type: 'importOk'; id: number; source: string }
  | { type: 'importErr'; id: number }
  | { type: 'scoreFormatted'; id: number; source: string }
  | { type: 'partOctaveShifted'; id: number; source: string }
  | {
      type: 'rangeOctaveShifted'
      id: number
      source: string
      /** The shifted note spans' own byte ranges in the *new* `source`,
       * synchronously computed by `source_edit::shift_range_octave` — lets
       * the caller restore the editor selection in the same tick it applies
       * the new source, closing a race where an async re-derivation could
       * lose to a second click landing first (see
       * `HANDOFF-octave-toolbar-part-label-selection-bug.md`). */
      ranges: { start: number; end: number }[]
    }
