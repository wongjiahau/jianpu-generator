//! Real bodies for the `Guest` methods `guest_impl.rs` dispatches to. Kept
//! at the exact by-value parameter types the `Guest` trait requires (see
//! `mod.rs`'s doc comment), so `needless_pass_by_value` is relaxed here.
#![allow(clippy::needless_pass_by_value)]

use super::*;

// Phase 3, group 7 of PLAN-wit-bindgen-migration.md: closes the real
// porting gap group 6's "fully complete" claim missed — these 17
// functions have live `web/src` call sites but no `#[wasm_bindgen] fn`
// counterpart existed anywhere in Phase 3's ordering list. Same
// underlying-function-reuse pattern as every prior group; see each
// method below for which existing `crate::` function it delegates to,
// and this file's/`wit/world.wit`'s comments for the handful that had no
// separate `crate::` function to delegate to at all (their logic was
// inline in the old `#[wasm_bindgen] fn` itself, called here directly
// rather than through a newly-extracted function, since duplicating a
// one-line call into a `jianpu_generator::` function is not meaningfully
// riskier than extracting it).

pub(super) fn get_metadata_defaults() -> MetadataDefaults {
    metadata_defaults_to_wit(&crate::metadata_types::MetadataDefaultsOut::default())
}

pub(super) fn get_default_lyrics_font_size(row_height: u32) -> u32 {
    jianpu_generator::ast::grouped::default_lyrics_font_size(row_height)
}

pub(super) fn get_default_title_font_size(row_height: u32) -> u32 {
    jianpu_generator::ast::grouped::default_title_font_size(row_height)
}

pub(super) fn get_default_subtitle_font_size(row_height: u32) -> u32 {
    jianpu_generator::ast::grouped::default_subtitle_font_size(row_height)
}

pub(super) fn get_default_author_font_size(row_height: u32) -> u32 {
    jianpu_generator::ast::grouped::default_author_font_size(row_height)
}

pub(super) fn get_default_part_legend_font_size(row_height: u32) -> u32 {
    jianpu_generator::ast::grouped::default_part_legend_font_size(row_height)
}

pub(super) fn get_default_page_number_font_size(row_height: u32) -> u32 {
    jianpu_generator::ast::grouped::default_page_number_font_size(row_height)
}

// Same underlying `jianpu_generator` calls as the old
// `wasm_boundary::set_layout_fonts`/`shift_part_octave`/`format_score`
// `#[wasm_bindgen] fn`s — their logic was already a direct call into
// `jianpu_generator`, not a separate `crate::` response function, so
// this duplicates that same one/three-line call rather than extracting
// a new shared function for it.

pub(super) fn set_layout_fonts(
    directive_line_font: Vec<u8>,
    lyric_font: Vec<u8>,
    monospace_font: Vec<u8>,
) {
    jianpu_generator::set_directive_line_font_bytes(directive_line_font);
    jianpu_generator::set_lyric_font_bytes(lyric_font);
    jianpu_generator::set_monospace_font_bytes(monospace_font);
}

pub(super) fn shift_part_octave(source: String, abbreviation: String, delta: i32) -> String {
    jianpu_generator::source_edit::shift_part_octave(&source, &abbreviation, delta as i8)
}

pub(super) fn shift_range_octave(
    source: String,
    ranges: Vec<ByteRange>,
    delta: i32,
) -> ShiftRangeOctaveResponse {
    let ranges: Vec<jianpu_generator::source_edit::ByteRange> = ranges
        .into_iter()
        .map(|range| jianpu_generator::source_edit::ByteRange {
            start_byte: range.start_byte,
            end_byte: range.end_byte,
        })
        .collect();
    let result = jianpu_generator::source_edit::shift_range_octave(&source, &ranges, delta as i8);
    ShiftRangeOctaveResponse {
        source: result.source,
        ranges: result
            .ranges
            .into_iter()
            .map(|range| ByteRange {
                start_byte: range.start_byte,
                end_byte: range.end_byte,
            })
            .collect(),
    }
}

pub(super) fn format_score(source: String) -> String {
    jianpu_generator::format_source::format_score(&source)
}

// Same underlying `crate::part_declarations::list_part_declarations_response`/
// `update_part_declaration_source` the old `wasm_boundary::
// list_part_declarations`/`update_part_declaration` `#[wasm_bindgen] fn`s
// call — untouched, both mechanisms coexist.

pub(super) fn list_part_declarations(
    source: String,
    raw_instruments: Vec<InstrumentInfo>,
) -> ListPartDeclarationsResponse {
    let instruments: Vec<jianpu_generator::parser::parts_parser::InstrumentInfo> = raw_instruments
        .into_iter()
        .map(instrument_info_from_wit)
        .collect();
    list_part_declarations_response_to_wit(
        crate::part_declarations::list_part_declarations_response(&source, &instruments),
    )
}

pub(super) fn update_part_declaration(
    source: String,
    abbreviation: String,
    new_mode: PartDeclarationMode,
    new_follow_target: String,
    new_soundfont: String,
    new_volume: String,
    new_octave_offset: String,
) -> String {
    crate::part_declarations::update_part_declaration_source(
        &source,
        &abbreviation,
        part_declaration_mode_from_wit(new_mode),
        &new_follow_target,
        &new_soundfont,
        &new_volume,
        &new_octave_offset,
    )
}

// `extract_source_from_svg`/`extract_source_from_pdf`'s logic was
// already a direct call into `jianpu_generator::source_embed` (no
// separate `crate::` response function existed), so these duplicate that
// same call rather than extracting a new shared function for it — same
// judgment call as `set_layout_fonts`/`shift_part_octave`/`format_score`
// above.

pub(super) fn extract_source_from_svg(svg_bytes: Vec<u8>) -> Option<String> {
    let svg = std::str::from_utf8(&svg_bytes).ok()?;
    jianpu_generator::source_embed::extract_embedded_source(svg)
}

pub(super) fn extract_source_from_pdf(pdf_bytes: Vec<u8>) -> Option<String> {
    jianpu_generator::source_embed::extract_embedded_source_from_pdf(&pdf_bytes)
}

// Same underlying `crate::compress_share_payload_bytes`/
// `decompress_share_payload_bytes` the old `share_payload::
// compress_share_payload`/`decompress_share_payload` `#[wasm_bindgen]
// fn`s now also call — both extracted out of `share_payload.rs` (a
// whole-module-gated file) into `lib.rs` this group, mirroring group
// 5/6's `types_export.rs`/`trim_window` extraction pattern, so this
// boundary can reach them too.

pub(super) fn compress_share_payload(payload: String) -> Vec<u8> {
    crate::compress_share_payload_bytes(&payload)
}

pub(super) fn decompress_share_payload(bytes: Vec<u8>) -> Option<String> {
    crate::decompress_share_payload_bytes(&bytes)
}

// Same underlying `crate::selection_range::resolve_selection_range_response`
// the old `selection_range::resolve_selection_range` `#[wasm_bindgen]
// fn` calls — that function was already unconditional (not gated on
// `wasm-bindgen-boundary`), unlike this group's other findings, so no
// prerequisite-ungating fix was needed here, only widening
// `ClickableElementId`/`NoteCellOut`/`LyricCellOut`/
// `ResolveSelectionRangeResponse`'s reachability from `component.rs` (see
// `selection_range/mod.rs`'s own comment on its `pub(crate) use`).

pub(super) fn resolve_selection_range(
    note_spans: Vec<NoteSpan>,
    lyric_spans: Vec<LyricSpan>,
    anchor: ClickableElementId,
    current: ClickableElementId,
) -> ResolveSelectionRangeResponse {
    let note_spans: Vec<crate::types::NoteSpanOut> =
        note_spans.into_iter().map(note_span_from_wit).collect();
    let lyric_spans: Vec<crate::types::LyricSpanOut> =
        lyric_spans.into_iter().map(lyric_span_from_wit).collect();
    let anchor = clickable_element_id_from_wit(anchor);
    let current = clickable_element_id_from_wit(current);
    resolve_selection_range_response_to_wit(
        crate::selection_range::resolve_selection_range_response(
            &note_spans,
            &lyric_spans,
            &anchor,
            &current,
        ),
    )
}
