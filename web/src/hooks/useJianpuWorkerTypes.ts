import type { RefObject } from 'react'
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

/** Tracks one in-flight "send source, get rewritten source back" round trip
 * to the render worker, so a stale reply (superseded by a newer request for
 * the same action) can be dropped instead of resolving out of order. */
export interface TextRequestTracker {
  requestIdRef: RefObject<number>
  latestIdRef: RefObject<number>
  pendingRequestsRef: RefObject<Map<number, (source: string) => void>>
}

/** Same tracking scheme as `TextRequestTracker`, but for
 * "shift selection octave" round trips, which resolve with the rewritten
 * source *and* the shifted notes' own byte ranges in that new source (see
 * `source_edit::shift_range_octave`) rather than just a plain string — the
 * caller needs both to restore the editor selection synchronously alongside
 * the new source, closing the race described in
 * `HANDOFF-octave-toolbar-part-label-selection-bug.md`. */
export interface RangeOctaveShiftRequestTracker {
  requestIdRef: RefObject<number>
  latestIdRef: RefObject<number>
  pendingRequestsRef: RefObject<
    Map<number, (result: { source: string; ranges: EditorSelection[] }) => void>
  >
}

/** One export's bytes, staged behind the rename-before-download modal until
 * the user confirms or cancels — see `useJianpuWorkerState.ts`'s
 * `requestDownload`/`confirmPendingDownload`/`cancelPendingDownload`. */
export interface PendingDownload {
  url: string
  filename: string
  /** Revoke `url` once the modal closes (confirmed or cancelled) — true for
   * one-shot export blobs (PDF/MIDI/zip) created solely for this download;
   * false for the WAV/MP3 preview's persistent object URL, which the
   * `<audio>` element still uses after the modal closes. */
  revokeOnClose: boolean
}

export interface JianpuWorkerState {
  parts: PartInfo[]
  partDeclarations: PartDeclaration[]
  partsLoading: boolean
  documents: SvgDocumentOut[]
  pendingDownload: PendingDownload | null
  /** Opens the rename-before-download modal for `url`/`filename` — see
   * `PendingDownload`'s `revokeOnClose` doc comment for what that flag
   * controls. Exposed for `App.tsx` to wire the WAV/MP3 preview player's
   * "Download" button, which builds its own object URL upstream of this
   * hook's own export message handlers. */
  requestDownload: (
    url: string,
    filename: string,
    revokeOnClose: boolean,
  ) => void
  confirmPendingDownload: (filename: string) => void
  cancelPendingDownload: () => void
  wavUrl: string | null
  wavFilename: string
  /** The full-score preview MP3 URL, mirroring `wavUrl` — mutually exclusive
   * with it (generating one revokes and clears the other), so `Preview`
   * shows whichever was most recently generated. */
  mp3Url: string | null
  mp3Filename: string
  /** Elapsed-seconds start/end of every sounding note/rest for `wavUrl`'s audio, keyed by `(source_part_index, note_id)`. Drives the per-part, per-note playback cursor. */
  noteTimings: NoteTimingOut[]
  audioAvailable: boolean
  pdfAvailable: boolean
  pdfExporting: boolean
  splitPdfExporting: boolean
  midiAvailable: boolean
  midiExporting: boolean
  splitMidiExporting: boolean
  splitWavExporting: boolean
  mp3Available: boolean
  mp3Exporting: boolean
  splitMp3Exporting: boolean
  diagnostics: Diagnostic[]
  diagnosticViewZones: DiagnosticViewZone[]
  rendering: boolean
  audioGenerating: boolean
  exportPdf: () => void
  exportSplitPdf: () => void
  exportMidi: () => void
  exportSplitMidi: () => void
  exportSplitWav: () => void
  exportMp3: () => void
  exportSplitMp3: () => void
  generateFullAudio: () => void
  selectedMeasureRange: {
    start: number
    end: number
    /** Which measure the preview should scroll to for this selection, when
     * it differs from `start` — a section/sequence chain selection can
     * resolve to a written-measure range whose start (in document order)
     * isn't where the user actually navigated to (e.g. dragging from an
     * early section down to a later one out of chain order). See
     * `notifySelection`'s `revealLine` parameter. */
    revealMeasureIndex: number
    /** The exact disjoint measure ranges to highlight in the SVG preview,
     * when they differ from the single `[start, end]` span above — a `#
     * sequence` chain selection spanning out-of-order entries (e.g. "C" and
     * a later repeat of "A", but not "B" in between). Only ever populated
     * by a sequence-chain selection; every other selection kind leaves this
     * unset and falls back to the caret-only single-range highlight. */
    highlightRanges?: { start: number; end: number }[]
  } | null
  measureAudioGenerating: boolean
  measureAudioPlaying: boolean
  /** Elapsed-seconds start/end of every sounding note/rest for the selected range's audio, keyed by `(source_part_index, note_id)`. */
  measureAudioNoteTimings: NoteTimingOut[]
  /** The `<audio>` element currently playing the selected measure range, if any; a new element each time playback starts. */
  measureAudioElement: HTMLAudioElement | null
  notifySelection: (
    startLine: number,
    endLine: number,
    isEmpty: boolean,
    /** The line the preview should scroll to, if it differs from
     * `startLine` — see `revealMeasureIndex` above. Defaults to
     * `startLine`. */
    revealLine?: number,
    /** The exact disjoint measure ranges to highlight in the SVG preview —
     * see `highlightRanges` above. */
    measureRanges?: { start: number; end: number }[],
  ) => void
  playSelectedMeasures: () => void
  playFromCurrentMeasure: () => void
  /** Plays only `selectedPartNames`, muting the rest, over
   * `[minMeasureIndex, maxMeasureIndex]`, then trims playback to the exact
   * elapsed-seconds window of `selectedCells` — see `useNoteSelection`'s
   * `selectedNoteRangePlaybackInfo`/`selectedNoteCells`. */
  playNoteSelection: (
    minMeasureIndex: number,
    maxMeasureIndex: number,
    selectedPartNames: string[],
    selectedCells: NoteCell[],
  ) => void
  /** Plays the whole score from its first measure through the last written
   * one, following any D.C./D.S./`# sequence` repeat structure — the "Play
   * All" button. */
  playAll: () => void
  stopMeasurePlayback: () => void
  highlightedDocuments: SvgDocumentOut[]
  measureSpans: MeasureSpan[]
  /** Source byte span of every note/chord/percussion/rest event, keyed by
   * `(source_part_index, note_id)` matching the SVG's `data-part-index`/
   * `data-note-id` attributes — see `useNoteSelection`. */
  noteSpans: NoteSpan[]
  /** Source byte span of every lyric syllable, keyed by
   * `(source_part_index, note_id, verse)` matching the SVG's
   * `data-part-index`/`data-note-id`/`data-verse` attributes — see
   * `useLyricSelection`. */
  lyricSpans: LyricSpan[]
  sectionRanges: SectionRange[]
  sequenceEntries: SequenceEntry[]
  /** The current mute/solo filter as sent to the `listNoteSpans`/
   * `listLyricSpans`/render worker messages — `undefined` means every part
   * is enabled. `undefined`-vs-empty-array (rather than always an array)
   * matches `enabledTracksForRender`'s contract. Needed by `useNoteSelection`
   * to resolve `sourcePartIndex` against the same visible-parts-only index
   * space `noteSpans` was compiled with. */
  enabledTracks: string[] | undefined
  previewInstrument: (programNumber: number) => void
  previewPercussion: (key: number) => void
  stopPreviewInstrument: () => void
  previewAudioPlaying: boolean
  updatePartDeclaration: (
    abbreviation: string,
    mode: PartMode,
    followTarget: string | null,
    soundfont: string | null,
    volume: number | null,
    octaveOffset: number | null,
  ) => Promise<string>
  /**
   * Zipped-view "Format" action: drops `# score` `[Key]` data lines that are
   * entirely redundant with implicit-fill, and collapses whitespace on
   * every surviving directive/data line (see `format_source::format_score`).
   */
  formatScore: (source: string) => Promise<string>
  /**
   * Rewrites the `'`/`,` octave marker on every note in the named part by
   * `delta` octaves (see `source_edit::shift_part_octave`) — the "notation
   * octave" control, distinct from the MIDI-only `octaveOffset` in
   * `updatePartDeclaration`. Resolves with the updated source; a `follow[X]`
   * part or unknown abbreviation resolves with the source unchanged.
   */
  shiftPartOctave: (abbreviation: string, delta: number) => Promise<string>
  /**
   * Rewrites the `'`/`,` octave marker on every note whose span overlaps
   * any of `ranges` by `delta` octaves (see
   * `source_edit::shift_range_octave`) — the editor toolbar's "shift
   * selection" octave-up/down action, scoped to the current selection
   * rather than a whole part. `ranges` is a disjoint set, not one min/max
   * span, so a multicursor selection (e.g. a clicked part label's notes
   * spanning every measure in its system) shifts every one of its pieces
   * without also sweeping in unrelated notes/parts sitting between them.
   * Resolves with the updated source and the shifted notes' own byte ranges
   * in that new source (see `source_edit::shift_range_octave`), so the
   * editor selection can be restored synchronously alongside the new
   * source instead of racing an async re-derivation.
   */
  shiftRangeOctave: (
    ranges: EditorSelection[],
    delta: number,
  ) => Promise<{ source: string; ranges: EditorSelection[] }>
  /**
   * Recovers the `.jianpu` source embedded in a previously exported SVG/PDF
   * file (see `source_embed::extract_embedded_source`). Rejects if the file
   * has no embedded source.
   */
  importFromFile: (file: File) => Promise<string>
}
