// Public API: render/list/group/symbol/part/metadata functions, matching
// the old (`tsify`-generated) `pkg/jianpu_wasm.d.ts` names/shapes exactly —
// split out of `jianpuWasm.ts` purely to stay under the 400-line-per-file
// cap. See `jianpuWasmApiAudio.ts` for the generate-audio/pdf/midi cluster.

import type {
  InstrumentInfo,
  LyricCellIn,
  LyricSpan as LyricSpanOut,
  MeasureRangeIn,
  NoteCellIn,
  NoteSpan as NoteSpanOut,
  GroupLyricSelectionResponse as WitGroupLyricSelectionResponse,
  GroupNoteSelectionResponse as WitGroupNoteSelectionResponse,
  ListLyricSpansResponse as WitListLyricSpansResponse,
  ListMeasureSpansResponse as WitListMeasureSpansResponse,
  ListNoteSpansResponse as WitListNoteSpansResponse,
  ListPartDeclarationsResponse as WitListPartDeclarationsResponse,
  ListPartsResponse as WitListPartsResponse,
  ListSymbolsResponse as WitListSymbolsResponse,
  MeasureAtOffsetResponse as WitMeasureAtOffsetResponse,
  PartDeclarationMode as WitPartDeclarationMode,
  RenameSymbolResponse as WitRenameSymbolResponse,
  RenderResponse as WitRenderResponse,
  ResolveSelectionRangeResponse as WitResolveSelectionRangeResponse,
} from '../../crates/jianpu-wasm/pkg-component/jianpu_wasm.js'
import type { ClickableElementId } from './components/clickableElementId'
import {
  convertClickableElementIdToWit,
  convertMeasureSpan,
  convertMetadataDefaults,
  convertPart,
  convertSectionRange,
  convertSequenceEntry,
  convertSvgDocument,
  convertSymbol,
  convertViewZone,
  diagnosticsErrOk,
  opt,
  SYMBOL_KIND_TO_WIT,
} from './jianpuWasmConvert'
import { root } from './jianpuWasmRoot'
import type {
  GroupLyricSelectionResponse,
  GroupNoteSelectionResponse,
  ListLyricSpansResponse,
  ListMeasureSpansResponse,
  ListNoteSpansResponse,
  ListPartDeclarationsResponse,
  ListPartsResponse,
  ListSymbolsResponse,
  MeasureAtOffsetResponse,
  MetadataDefaultsOut,
  RenameSymbolResponse,
  RenderResponse,
  ResolveSelectionRangeResponse,
  SymbolKindOut,
} from './jianpuWasmTypes'

// ==================== public API (old function names/shapes) ====================

export function render(
  source: string,
  enabled_tracks: string[] | null | undefined,
  disabled_lyrics: string[] | null | undefined,
  raw_instruments: InstrumentInfo[],
): RenderResponse {
  const resp: WitRenderResponse = root().renderSvg(
    source,
    opt(enabled_tracks),
    opt(disabled_lyrics),
    raw_instruments,
  )
  if (resp.tag === 'ok') {
    return {
      status: 'ok',
      documents: resp.val.documents.map(convertSvgDocument),
      diagnostics: resp.val.diagnostics,
      diagnostic_view_zones: resp.val.diagnosticViewZones.map(convertViewZone),
    }
  }
  return {
    status: 'err',
    diagnostics: resp.val.diagnostics,
    diagnostic_view_zones: resp.val.diagnosticViewZones.map(convertViewZone),
  }
}

export function render_with_highlight_range(
  source: string,
  raw_measure_ranges: MeasureRangeIn[],
  enabled_tracks: string[] | null | undefined,
  disabled_lyrics: string[] | null | undefined,
  raw_instruments: InstrumentInfo[],
): RenderResponse {
  const resp: WitRenderResponse = root().renderSvgWithHighlightRange(
    source,
    raw_measure_ranges,
    opt(enabled_tracks),
    opt(disabled_lyrics),
    raw_instruments,
  )
  if (resp.tag === 'ok') {
    return {
      status: 'ok',
      documents: resp.val.documents.map(convertSvgDocument),
      diagnostics: resp.val.diagnostics,
      diagnostic_view_zones: resp.val.diagnosticViewZones.map(convertViewZone),
    }
  }
  return {
    status: 'err',
    diagnostics: resp.val.diagnostics,
    diagnostic_view_zones: resp.val.diagnosticViewZones.map(convertViewZone),
  }
}

export function list_parts(
  source: string,
  raw_instruments: InstrumentInfo[],
): ListPartsResponse {
  const resp: WitListPartsResponse = root().listParts(source, raw_instruments)
  return diagnosticsErrOk(resp, (v) => ({
    parts: v.parts.map(convertPart),
    declarations: v.declarations,
  }))
}

export function list_part_declarations(
  source: string,
  raw_instruments: InstrumentInfo[],
): ListPartDeclarationsResponse {
  const resp: WitListPartDeclarationsResponse = root().listPartDeclarations(
    source,
    raw_instruments,
  )
  return diagnosticsErrOk(resp, (v) => ({ declarations: v.declarations }))
}

export function update_part_declaration(
  source: string,
  abbreviation: string,
  new_mode: WitPartDeclarationMode,
  new_follow_target: string,
  new_soundfont: string,
  new_volume: string,
  new_octave_offset: string,
): string {
  return root().updatePartDeclaration(
    source,
    abbreviation,
    new_mode,
    new_follow_target,
    new_soundfont,
    new_volume,
    new_octave_offset,
  )
}

export function list_symbols(
  source: string,
  raw_instruments: InstrumentInfo[],
): ListSymbolsResponse {
  const resp: WitListSymbolsResponse = root().listSymbols(
    source,
    raw_instruments,
  )
  return diagnosticsErrOk(resp, (v) => ({
    symbols: v.symbols.map(convertSymbol),
  }))
}

export function rename_symbol(
  source: string,
  kind: SymbolKindOut,
  old_name: string,
  new_name: string,
  raw_instruments: InstrumentInfo[],
): RenameSymbolResponse {
  const resp: WitRenameSymbolResponse = root().renameSymbol(
    source,
    SYMBOL_KIND_TO_WIT[kind],
    old_name,
    new_name,
    raw_instruments,
  )
  return diagnosticsErrOk(resp, (v) => ({ edits: v.edits }))
}

export function get_measure_index_at_offset(
  source: string,
  byte_offset: number,
): MeasureAtOffsetResponse {
  const resp: WitMeasureAtOffsetResponse = root().getMeasureIndexAtOffset(
    source,
    byte_offset,
  )
  if (resp.tag === 'ok') {
    return { status: 'ok', measure_index: resp.val.measureIndex }
  }
  return { status: 'notInMeasure' }
}

export function list_note_spans(
  source: string,
  enabled_tracks?: string[] | null,
): ListNoteSpansResponse {
  const resp: WitListNoteSpansResponse = root().listNoteSpans(
    source,
    opt(enabled_tracks),
  )
  if (resp.tag === 'ok') return { status: 'ok', spans: resp.val.spans }
  return { status: 'err' }
}

export function list_lyric_spans(
  source: string,
  enabled_tracks?: string[] | null,
): ListLyricSpansResponse {
  const resp: WitListLyricSpansResponse = root().listLyricSpans(
    source,
    opt(enabled_tracks),
  )
  if (resp.tag === 'ok') return { status: 'ok', spans: resp.val.spans }
  return { status: 'err' }
}

export function list_measure_spans(source: string): ListMeasureSpansResponse {
  const resp: WitListMeasureSpansResponse = root().listMeasureSpans(source)
  if (resp.tag === 'ok') {
    return {
      status: 'ok',
      spans: resp.val.spans.map(convertMeasureSpan),
      section_ranges: resp.val.sectionRanges.map(convertSectionRange),
      sequence_entries: resp.val.sequenceEntries.map(convertSequenceEntry),
    }
  }
  return { status: 'err' }
}

export function group_note_selection(
  raw_note_spans: NoteSpanOut[],
  raw_selected_cells: NoteCellIn[],
): GroupNoteSelectionResponse {
  const resp: WitGroupNoteSelectionResponse = root().groupNoteSelection(
    raw_note_spans,
    raw_selected_cells,
  )
  if (resp.tag === 'ok') return { status: 'ok', runs: resp.val.runs }
  return { status: 'err' }
}

export function group_lyric_selection(
  raw_lyric_spans: LyricSpanOut[],
  raw_selected_cells: LyricCellIn[],
): GroupLyricSelectionResponse {
  const resp: WitGroupLyricSelectionResponse = root().groupLyricSelection(
    raw_lyric_spans,
    raw_selected_cells,
  )
  if (resp.tag === 'ok') return { status: 'ok', runs: resp.val.runs }
  return { status: 'err' }
}

export function get_metadata_defaults(): MetadataDefaultsOut {
  return convertMetadataDefaults(root().getMetadataDefaults())
}

export function get_default_lyrics_font_size(row_height: number): number {
  return root().getDefaultLyricsFontSize(row_height)
}

export function get_default_title_font_size(row_height: number): number {
  return root().getDefaultTitleFontSize(row_height)
}

export function get_default_subtitle_font_size(row_height: number): number {
  return root().getDefaultSubtitleFontSize(row_height)
}

export function get_default_author_font_size(row_height: number): number {
  return root().getDefaultAuthorFontSize(row_height)
}

export function get_default_part_legend_font_size(row_height: number): number {
  return root().getDefaultPartLegendFontSize(row_height)
}

export function get_default_page_number_font_size(row_height: number): number {
  return root().getDefaultPageNumberFontSize(row_height)
}

export function set_layout_fonts(
  directive_line_font: Uint8Array,
  lyric_font: Uint8Array,
  monospace_font: Uint8Array,
): void {
  root().setLayoutFonts(directive_line_font, lyric_font, monospace_font)
}

export function shift_part_octave(
  source: string,
  abbreviation: string,
  delta: number,
): string {
  return root().shiftPartOctave(source, abbreviation, delta)
}

export function shift_range_octave(
  source: string,
  ranges: Array<{ start: number; end: number }>,
  delta: number,
): { source: string; ranges: Array<{ start: number; end: number }> } {
  const resp = root().shiftRangeOctave(
    source,
    ranges.map((range) => ({ startByte: range.start, endByte: range.end })),
    delta,
  )
  return {
    source: resp.source,
    ranges: resp.ranges.map((range) => ({
      start: range.startByte,
      end: range.endByte,
    })),
  }
}

export function format_score(source: string): string {
  return root().formatScore(source)
}

export function resolve_selection_range(
  raw_note_spans: NoteSpanOut[],
  raw_lyric_spans: LyricSpanOut[],
  raw_anchor: ClickableElementId,
  raw_current: ClickableElementId,
): ResolveSelectionRangeResponse {
  const resp: WitResolveSelectionRangeResponse = root().resolveSelectionRange(
    raw_note_spans,
    raw_lyric_spans,
    convertClickableElementIdToWit(raw_anchor),
    convertClickableElementIdToWit(raw_current),
  )
  if (resp.tag === 'ok') {
    return {
      status: 'ok',
      note_cells: resp.val.noteCells,
      lyric_cells: resp.val.lyricCells,
    }
  }
  return { status: 'err' }
}
