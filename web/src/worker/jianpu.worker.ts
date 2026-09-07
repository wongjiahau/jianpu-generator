import * as jianpuWasm from '../jianpuWasm'
import {
  extract_source_from_pdf,
  extract_source_from_svg,
  format_score,
  generate_instrument_preview_wav as generateInstrumentPreviewWav,
  generate_midi as generateMidi,
  generate_mp3 as generateMp3,
  generate_pdf as generatePdf,
  generate_percussion_preview_wav as generatePercussionPreviewWav,
  generate_split_midis as generateSplitMidis,
  generate_split_mp3s as generateSplitMp3s,
  generate_split_pdfs as generateSplitPdfs,
  generate_split_wavs as generateSplitWavs,
  generate_wav as generateWav,
  generate_wav_for_measure_range as generateWavForMeasureRange,
  list_lyric_spans,
  list_measure_spans,
  list_note_spans,
  list_parts,
  list_note_timings as listNoteTimings,
  list_note_timings_for_range as listNoteTimingsForRange,
  render,
  render_with_highlight_range as renderWithHighlightRange,
  set_layout_fonts,
  shift_part_octave,
  shift_range_octave,
  update_part_declaration,
} from '../jianpuWasm'
import type { PartDeclaration } from '../types'
import { GM_INSTRUMENTS } from '../utils/gmInstruments'
import { instantiateWasmComponentFromModule } from '../wasmInit'
import {
  handleGenerateAudio,
  handleGenerateMeasureRangeAudio,
  handleGenerateMp3,
} from './audioMessageHandlers'
import {
  handleGenerateMidi,
  handleGeneratePdf,
  handleGenerateSplitMidi,
  handleGenerateSplitMp3,
  handleGenerateSplitPdf,
  handleGenerateSplitWav,
} from './exportMessageHandlers'
import { handleImportFromFile } from './importMessageHandlers'
import type { WorkerRequest, WorkerResponse } from './messages'
import {
  handlePreviewInstrument,
  handlePreviewPercussion,
} from './previewMessageHandlers'

export type { WorkerRequest, WorkerResponse } from './messages'

let resolveWasmModule: (module: WebAssembly.Module) => void
const wasmModulePromise = new Promise<WebAssembly.Module>((resolve) => {
  resolveWasmModule = resolve
})

let initPromise: Promise<void> | null = null

function ensureInit(): Promise<void> {
  if (!initPromise) {
    initPromise = wasmModulePromise
      .then((module) => instantiateWasmComponentFromModule(module))
      .then((root) => {
        jianpuWasm.setWasmRoot(root)
        postMessage({
          type: 'ready',
          audioAvailable: true,
          pdfAvailable: true,
          midiAvailable: true,
          mp3Available: true,
        } satisfies WorkerResponse)
      })
  }
  return initPromise
}

// Applies the layout fonts to the wasm module as soon as they've both
// arrived (from the `loadPdfFonts` message, reusing the bytes the app
// already fetches for PDF export) and the module is ready to receive them.
// Deliberately not awaited by any render: renders that happen before the
// fonts land just use the character-bucket fallback for that render, the
// same graceful degradation as a font fetch that fails outright. Blocking
// render on a network fetch would turn a slow or failed fetch into a stuck
// preview instead of a merely imprecise one.
function applyCoreFontsWhenReady(fonts: {
  sc: Uint8Array
  tc: Uint8Array
  mono: Uint8Array
}): void {
  // `set_layout_fonts(directive_line_font, lyric_font, monospace_font)` —
  // directive-line text measures against `tc` (the `sansSerif` role's
  // font), lyrics against `sc` (the `serif` role's font, shared with the
  // song title) — see `fonts/fonts.json` and
  // `DIRECTIVE_LINE_FONT_FAMILY`/`SERIF_FONT_FAMILY` in
  // src/serializer/mod.rs.
  ensureInit().then(() => set_layout_fonts(fonts.tc, fonts.sc, fonts.mono))
}

let loadedSoundfont: Uint8Array | null = null
// `sc` holds the `serif` role's font — the song title/lyric font; `tc`
// holds the `sansSerif` role's font, the default/body font for everything
// else — see `fonts/fonts.json` and `useFontsLoader`.
let loadedFonts: {
  sc: Uint8Array
  tc: Uint8Array
  mono: Uint8Array
} | null = null

function octaveOffsetToWasmString(octaveOffset: number | null): string {
  if (octaveOffset == null || octaveOffset === 0) return ''
  return octaveOffset > 0 ? `+${octaveOffset}` : String(octaveOffset)
}

function listDeclarationsFromSource(source: string): PartDeclaration[] {
  const result = jianpuWasm.list_part_declarations(source, GM_INSTRUMENTS)
  return result.status === 'ok' ? result.declarations : []
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const msg = event.data

  if (msg.type === 'wasmModule') {
    resolveWasmModule(msg.module)
    return
  }

  if (msg.type === 'loadSoundfont') {
    loadedSoundfont = new Uint8Array(msg.soundfont)
    return
  }

  if (msg.type === 'loadPdfFonts') {
    const sc = new Uint8Array(msg.scFont)
    const tc = new Uint8Array(msg.tcFont)
    const mono = new Uint8Array(msg.monoFont)
    loadedFonts = { sc, tc, mono }
    applyCoreFontsWhenReady({ sc, tc, mono })
    return
  }

  await ensureInit()

  if (msg.type === 'listParts') {
    const result = list_parts(msg.source, GM_INSTRUMENTS)
    if (result.status === 'ok') {
      postMessage({
        type: 'parts',
        id: msg.id,
        parts: result.parts,
        declarations: result.declarations,
      } satisfies WorkerResponse)
      return
    }

    postMessage({
      type: 'parts',
      id: msg.id,
      parts: [],
      declarations: [],
    } satisfies WorkerResponse)
    return
  }

  if (msg.type === 'updatePartDeclaration') {
    const newSource = update_part_declaration(
      msg.source,
      msg.abbreviation,
      msg.mode,
      msg.followTarget ?? '',
      msg.soundfont ?? '',
      msg.volume != null ? String(msg.volume) : '',
      octaveOffsetToWasmString(msg.octaveOffset),
    )
    postMessage({
      type: 'partDeclarationUpdated',
      id: msg.id,
      source: newSource,
      declarations: listDeclarationsFromSource(newSource),
    } satisfies WorkerResponse)
    return
  }

  if (msg.type === 'formatScore') {
    postMessage({
      type: 'scoreFormatted',
      id: msg.id,
      source: format_score(msg.source),
    } satisfies WorkerResponse)
    return
  }

  if (msg.type === 'shiftPartOctave') {
    postMessage({
      type: 'partOctaveShifted',
      id: msg.id,
      source: shift_part_octave(msg.source, msg.abbreviation, msg.delta),
    } satisfies WorkerResponse)
    return
  }

  if (msg.type === 'shiftRangeOctave') {
    const result = shift_range_octave(msg.source, msg.ranges, msg.delta)
    postMessage({
      type: 'rangeOctaveShifted',
      id: msg.id,
      source: result.source,
      ranges: result.ranges,
    } satisfies WorkerResponse)
    return
  }

  if (msg.type === 'generatePdf') {
    handleGeneratePdf(msg, generatePdf, loadedFonts)
    return
  }

  if (msg.type === 'generateSplitPdf') {
    handleGenerateSplitPdf(msg, generateSplitPdfs, loadedFonts)
    return
  }

  if (msg.type === 'generateMidi') {
    handleGenerateMidi(msg, generateMidi)
    return
  }

  if (msg.type === 'generateSplitMidi') {
    handleGenerateSplitMidi(msg, generateSplitMidis)
    return
  }

  if (msg.type === 'generateSplitWav') {
    handleGenerateSplitWav(msg, generateSplitWavs, loadedSoundfont)
    return
  }

  if (msg.type === 'generateMp3') {
    handleGenerateMp3(msg, generateMp3, listNoteTimings, loadedSoundfont)
    return
  }

  if (msg.type === 'generateSplitMp3') {
    handleGenerateSplitMp3(msg, generateSplitMp3s, loadedSoundfont)
    return
  }

  if (msg.type === 'generateAudio') {
    handleGenerateAudio(msg, generateWav, listNoteTimings, loadedSoundfont)
    return
  }

  if (msg.type === 'generateMeasureRangeAudio') {
    handleGenerateMeasureRangeAudio(
      msg,
      generateWavForMeasureRange,
      listNoteTimingsForRange,
      loadedSoundfont,
    )
    return
  }

  if (msg.type === 'previewInstrument') {
    handlePreviewInstrument(msg, generateInstrumentPreviewWav, loadedSoundfont)
    return
  }

  if (msg.type === 'previewPercussion') {
    handlePreviewPercussion(msg, generatePercussionPreviewWav, loadedSoundfont)
    return
  }

  if (msg.type === 'renderWithHighlightRange') {
    const result = renderWithHighlightRange(
      msg.source,
      msg.ranges,
      msg.enabledTracks,
      msg.disabledLyrics,
      GM_INSTRUMENTS,
    )
    if (result.status === 'ok') {
      postMessage({
        type: 'highlightRangeOk',
        id: msg.id,
        documents: result.documents,
      } satisfies WorkerResponse)
      return
    }
    postMessage({
      type: 'highlightRangeErr',
      id: msg.id,
      diagnostics: result.diagnostics,
    } satisfies WorkerResponse)
    return
  }

  if (msg.type === 'importFromFile') {
    handleImportFromFile(msg, extract_source_from_svg, extract_source_from_pdf)
    return
  }

  if (msg.type === 'listMeasureSpans') {
    const result = list_measure_spans(msg.source)
    postMessage({
      type: 'measureSpans',
      id: msg.id,
      status: result.status,
      spans: result.status === 'ok' ? result.spans : [],
      sectionRanges: result.status === 'ok' ? result.section_ranges : [],
      sequenceEntries: result.status === 'ok' ? result.sequence_entries : [],
    } satisfies WorkerResponse)
    return
  }

  if (msg.type === 'listNoteSpans') {
    const result = list_note_spans(msg.source, msg.enabledTracks)
    postMessage({
      type: 'noteSpans',
      id: msg.id,
      status: result.status,
      spans: result.status === 'ok' ? result.spans : [],
    } satisfies WorkerResponse)
    return
  }

  if (msg.type === 'listLyricSpans') {
    const result = list_lyric_spans(msg.source, msg.enabledTracks)
    postMessage({
      type: 'lyricSpans',
      id: msg.id,
      status: result.status,
      spans: result.status === 'ok' ? result.spans : [],
    } satisfies WorkerResponse)
    return
  }

  if (msg.type !== 'render') return

  const result = render(
    msg.source,
    msg.enabledTracks,
    msg.disabledLyrics,
    GM_INSTRUMENTS,
  )
  if (result.status === 'ok') {
    postMessage({
      type: 'ok',
      id: msg.id,
      documents: result.documents,
      diagnostics: result.diagnostics,
      diagnosticViewZones: result.diagnostic_view_zones,
    } satisfies WorkerResponse)
    return
  }

  postMessage({
    type: 'err',
    id: msg.id,
    diagnostics: result.diagnostics,
    diagnosticViewZones: result.diagnostic_view_zones,
  } satisfies WorkerResponse)
}
