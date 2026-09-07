//! The single `impl Guest for Component` block (Rust forbids splitting
//! one trait impl across files). Each method's real logic — and the
//! `PLAN-wit-bindgen-migration.md` "Phase N" comment describing it — lives
//! in the same-named function in the relevant `guest_*` submodule; this
//! file is just the trait wiring.
use super::*;

impl Guest for Component {
    fn greet(name: String) -> String {
        greet(name)
    }

    fn group_note_selection(
        note_spans: Vec<NoteSpan>,
        selected_cells: Vec<NoteCellIn>,
    ) -> GroupNoteSelectionResponse {
        group_note_selection(note_spans, selected_cells)
    }

    fn group_lyric_selection(
        lyric_spans: Vec<LyricSpan>,
        selected_cells: Vec<LyricCellIn>,
    ) -> GroupLyricSelectionResponse {
        group_lyric_selection(lyric_spans, selected_cells)
    }

    fn list_measure_spans(source: String) -> ListMeasureSpansResponse {
        list_measure_spans(source)
    }

    fn list_note_spans(
        source: String,
        enabled_tracks: Option<Vec<String>>,
    ) -> ListNoteSpansResponse {
        list_note_spans(source, enabled_tracks)
    }

    fn list_lyric_spans(
        source: String,
        enabled_tracks: Option<Vec<String>>,
    ) -> ListLyricSpansResponse {
        list_lyric_spans(source, enabled_tracks)
    }

    fn list_parts(source: String, raw_instruments: Vec<InstrumentInfo>) -> ListPartsResponse {
        list_parts(source, raw_instruments)
    }

    fn list_symbols(source: String, raw_instruments: Vec<InstrumentInfo>) -> ListSymbolsResponse {
        list_symbols(source, raw_instruments)
    }

    fn rename_symbol(
        source: String,
        kind: SymbolKind,
        old_name: String,
        new_name: String,
        raw_instruments: Vec<InstrumentInfo>,
    ) -> RenameSymbolResponse {
        rename_symbol(source, kind, old_name, new_name, raw_instruments)
    }

    fn get_measure_index_at_offset(source: String, byte_offset: u32) -> MeasureAtOffsetResponse {
        get_measure_index_at_offset(source, byte_offset)
    }

    fn render_svg(
        source: String,
        enabled_tracks: Option<Vec<String>>,
        disabled_lyrics: Option<Vec<String>>,
        raw_instruments: Vec<InstrumentInfo>,
    ) -> RenderResponse {
        render_svg(source, enabled_tracks, disabled_lyrics, raw_instruments)
    }

    fn render_svg_with_highlight_range(
        source: String,
        raw_measure_ranges: Vec<MeasureRangeIn>,
        enabled_tracks: Option<Vec<String>>,
        disabled_lyrics: Option<Vec<String>>,
        raw_instruments: Vec<InstrumentInfo>,
    ) -> RenderResponse {
        render_svg_with_highlight_range(
            source,
            raw_measure_ranges,
            enabled_tracks,
            disabled_lyrics,
            raw_instruments,
        )
    }

    fn generate_wav(
        source: String,
        enabled_tracks: Option<Vec<String>>,
        soundfont: Vec<u8>,
    ) -> GenerateWavResponse {
        generate_wav(source, enabled_tracks, soundfont)
    }

    fn generate_split_wavs(
        source: String,
        base_name: String,
        soundfont: Vec<u8>,
    ) -> GenerateSplitWavsResponse {
        generate_split_wavs(source, base_name, soundfont)
    }

    fn generate_mp3(
        source: String,
        enabled_tracks: Option<Vec<String>>,
        soundfont: Vec<u8>,
    ) -> GenerateMp3Response {
        generate_mp3(source, enabled_tracks, soundfont)
    }

    fn generate_split_mp3s(
        source: String,
        base_name: String,
        soundfont: Vec<u8>,
    ) -> GenerateSplitMp3sResponse {
        generate_split_mp3s(source, base_name, soundfont)
    }

    fn generate_pdf(
        source: String,
        enabled_tracks: Option<Vec<String>>,
        disabled_lyrics: Option<Vec<String>>,
        sans_serif_sc: Vec<u8>,
        sans_serif_tc: Vec<u8>,
        monospace: Vec<u8>,
    ) -> GeneratePdfResponse {
        generate_pdf(
            source,
            enabled_tracks,
            disabled_lyrics,
            sans_serif_sc,
            sans_serif_tc,
            monospace,
        )
    }

    fn generate_split_pdfs(
        source: String,
        base_name: String,
        sans_serif_sc: Vec<u8>,
        sans_serif_tc: Vec<u8>,
        monospace: Vec<u8>,
    ) -> GenerateSplitPdfsResponse {
        generate_split_pdfs(source, base_name, sans_serif_sc, sans_serif_tc, monospace)
    }

    fn generate_midi(source: String, enabled_tracks: Option<Vec<String>>) -> GenerateMidiResponse {
        generate_midi(source, enabled_tracks)
    }

    fn generate_split_midis(source: String, base_name: String) -> GenerateSplitMidisResponse {
        generate_split_midis(source, base_name)
    }

    fn generate_wav_for_measure_range(
        source: String,
        start_index: u32,
        end_index: u32,
        extend_to_last_occurrence: bool,
        respect_sequence: bool,
        sequence_entry_start_index: Option<u32>,
        sequence_entry_end_index: Option<u32>,
        enabled_tracks: Option<Vec<String>>,
        trim_start_s: Option<f64>,
        trim_end_s: Option<f64>,
        trim_next_note_start_s: Option<f64>,
        soundfont: Vec<u8>,
    ) -> GenerateWavResponse {
        generate_wav_for_measure_range(
            source,
            start_index,
            end_index,
            extend_to_last_occurrence,
            respect_sequence,
            sequence_entry_start_index,
            sequence_entry_end_index,
            enabled_tracks,
            trim_start_s,
            trim_end_s,
            trim_next_note_start_s,
            soundfont,
        )
    }

    fn generate_mp3_for_measure_range(
        source: String,
        start_index: u32,
        end_index: u32,
        extend_to_last_occurrence: bool,
        respect_sequence: bool,
        sequence_entry_start_index: Option<u32>,
        sequence_entry_end_index: Option<u32>,
        enabled_tracks: Option<Vec<String>>,
        trim_start_s: Option<f64>,
        trim_end_s: Option<f64>,
        trim_next_note_start_s: Option<f64>,
        soundfont: Vec<u8>,
    ) -> GenerateMp3Response {
        generate_mp3_for_measure_range(
            source,
            start_index,
            end_index,
            extend_to_last_occurrence,
            respect_sequence,
            sequence_entry_start_index,
            sequence_entry_end_index,
            enabled_tracks,
            trim_start_s,
            trim_end_s,
            trim_next_note_start_s,
            soundfont,
        )
    }

    fn list_note_timings(
        source: String,
        visible_tracks: Option<Vec<String>>,
        enabled_tracks: Option<Vec<String>>,
    ) -> NoteTimingsResponse {
        list_note_timings(source, visible_tracks, enabled_tracks)
    }

    fn list_note_timings_for_range(
        source: String,
        start_index: u32,
        end_index: u32,
        extend_to_last_occurrence: bool,
        respect_sequence: bool,
        sequence_entry_start_index: Option<u32>,
        sequence_entry_end_index: Option<u32>,
        visible_tracks: Option<Vec<String>>,
        enabled_tracks: Option<Vec<String>>,
    ) -> NoteTimingsResponse {
        list_note_timings_for_range(
            source,
            start_index,
            end_index,
            extend_to_last_occurrence,
            respect_sequence,
            sequence_entry_start_index,
            sequence_entry_end_index,
            visible_tracks,
            enabled_tracks,
        )
    }

    fn generate_instrument_preview_wav(
        program_number: u8,
        soundfont: Vec<u8>,
    ) -> GenerateWavResponse {
        generate_instrument_preview_wav(program_number, soundfont)
    }

    fn generate_percussion_preview_wav(key: u8, soundfont: Vec<u8>) -> GenerateWavResponse {
        generate_percussion_preview_wav(key, soundfont)
    }

    fn get_metadata_defaults() -> MetadataDefaults {
        get_metadata_defaults()
    }

    fn get_default_lyrics_font_size(row_height: u32) -> u32 {
        get_default_lyrics_font_size(row_height)
    }

    fn get_default_title_font_size(row_height: u32) -> u32 {
        get_default_title_font_size(row_height)
    }

    fn get_default_subtitle_font_size(row_height: u32) -> u32 {
        get_default_subtitle_font_size(row_height)
    }

    fn get_default_author_font_size(row_height: u32) -> u32 {
        get_default_author_font_size(row_height)
    }

    fn get_default_part_legend_font_size(row_height: u32) -> u32 {
        get_default_part_legend_font_size(row_height)
    }

    fn get_default_page_number_font_size(row_height: u32) -> u32 {
        get_default_page_number_font_size(row_height)
    }

    fn set_layout_fonts(
        directive_line_font: Vec<u8>,
        lyric_font: Vec<u8>,
        monospace_font: Vec<u8>,
    ) {
        set_layout_fonts(directive_line_font, lyric_font, monospace_font)
    }

    fn shift_part_octave(source: String, abbreviation: String, delta: i32) -> String {
        shift_part_octave(source, abbreviation, delta)
    }

    fn shift_range_octave(
        source: String,
        ranges: Vec<ByteRange>,
        delta: i32,
    ) -> ShiftRangeOctaveResponse {
        shift_range_octave(source, ranges, delta)
    }

    fn format_score(source: String) -> String {
        format_score(source)
    }

    fn list_part_declarations(
        source: String,
        raw_instruments: Vec<InstrumentInfo>,
    ) -> ListPartDeclarationsResponse {
        list_part_declarations(source, raw_instruments)
    }

    fn update_part_declaration(
        source: String,
        abbreviation: String,
        new_mode: PartDeclarationMode,
        new_follow_target: String,
        new_soundfont: String,
        new_volume: String,
        new_octave_offset: String,
    ) -> String {
        update_part_declaration(
            source,
            abbreviation,
            new_mode,
            new_follow_target,
            new_soundfont,
            new_volume,
            new_octave_offset,
        )
    }

    fn extract_source_from_svg(svg_bytes: Vec<u8>) -> Option<String> {
        extract_source_from_svg(svg_bytes)
    }

    fn extract_source_from_pdf(pdf_bytes: Vec<u8>) -> Option<String> {
        extract_source_from_pdf(pdf_bytes)
    }

    fn compress_share_payload(payload: String) -> Vec<u8> {
        compress_share_payload(payload)
    }

    fn decompress_share_payload(bytes: Vec<u8>) -> Option<String> {
        decompress_share_payload(bytes)
    }

    fn resolve_selection_range(
        note_spans: Vec<NoteSpan>,
        lyric_spans: Vec<LyricSpan>,
        anchor: ClickableElementId,
        current: ClickableElementId,
    ) -> ResolveSelectionRangeResponse {
        resolve_selection_range(note_spans, lyric_spans, anchor, current)
    }
}
