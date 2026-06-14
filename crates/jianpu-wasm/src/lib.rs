mod types;

#[cfg(feature = "wav")]
use jianpu_generator::write_wav_for_measure_from_source;
#[cfg(feature = "wav")]
use jianpu_generator::write_wav_from_source_filtered;
use jianpu_generator::{
    compile, list_parts_from_source,
    render_svgs_from_source_filtered_with_lyrics,
};
#[cfg(feature = "pdf")]
use jianpu_generator::{
    write_pdf_from_source_filtered_with_lyrics, write_split_pdfs_from_source, zip_split_pdfs,
};
#[cfg(feature = "wav")]
use types::GenerateWavResponse;
use types::{
    diagnostic_from_error, to_js_value, ListPartsResponse, MeasureAtOffsetResponse, PartOut,
    RenderResponse,
};
#[cfg(feature = "pdf")]
use types::{GeneratePdfResponse, GenerateSplitPdfsResponse};
use wasm_bindgen::prelude::*;

fn render_response(
    source: &str,
    enabled_tracks: Option<Vec<String>>,
    disabled_lyrics: Option<Vec<String>>,
) -> RenderResponse {
    let tracks = enabled_tracks.as_deref();
    let lyrics = disabled_lyrics.as_deref();
    match render_svgs_from_source_filtered_with_lyrics(source, "input.jianpu", tracks, lyrics) {
        Ok(svgs) => RenderResponse::Ok { svgs },
        Err(e) => RenderResponse::Err {
            diagnostics: vec![diagnostic_from_error(source, e)],
        },
    }
}

fn list_parts_response(source: &str) -> ListPartsResponse {
    match list_parts_from_source(source, "input.jianpu") {
        Ok(parts) => ListPartsResponse::Ok {
            parts: parts
                .into_iter()
                .map(|part| PartOut {
                    abbreviation: part.abbreviation,
                    display_name: part.display_name,
                    has_lyrics: part.has_lyrics,
                })
                .collect(),
        },
        Err(e) => ListPartsResponse::Err {
            diagnostics: vec![diagnostic_from_error(source, e)],
        },
    }
}

fn get_measure_at_offset_response(source: &str, byte_offset: usize) -> MeasureAtOffsetResponse {
    match compile(source, "input.jianpu") {
        Ok(score) => {
            let clamped = byte_offset.min(source.len());
            let target_line = source[..clamped].bytes().filter(|&b| b == b'\n').count();
            let index = score.measures.iter().position(|m| {
                let start_line = source[..m.source_span.start].bytes().filter(|&b| b == b'\n').count();
                let end = m.source_span.end.min(source.len());
                let end_line = source[..end].bytes().filter(|&b| b == b'\n').count();
                start_line <= target_line && target_line <= end_line
            });
            match index {
                Some(i) => MeasureAtOffsetResponse::Ok { measure_index: i },
                None => MeasureAtOffsetResponse::NotInMeasure,
            }
        }
        Err(_) => MeasureAtOffsetResponse::NotInMeasure,
    }
}

#[cfg(feature = "wav")]
fn generate_wav_response(source: &str, enabled_tracks: Option<Vec<String>>) -> GenerateWavResponse {
    let tracks = enabled_tracks.as_deref();
    match write_wav_from_source_filtered(source, "input.jianpu", tracks) {
        Ok(wav) => GenerateWavResponse::Ok { wav },
        Err(e) => GenerateWavResponse::Err {
            diagnostics: vec![diagnostic_from_error(source, e)],
        },
    }
}

#[cfg(feature = "wav")]
fn generate_wav_for_measure_response(
    source: &str,
    measure_index: usize,
    enabled_tracks: Option<Vec<String>>,
) -> GenerateWavResponse {
    let tracks = enabled_tracks.as_deref();
    match write_wav_for_measure_from_source(source, "input.jianpu", measure_index, tracks) {
        Ok(wav) => GenerateWavResponse::Ok { wav },
        Err(e) => GenerateWavResponse::Err {
            diagnostics: vec![diagnostic_from_error(source, e)],
        },
    }
}

#[cfg(feature = "pdf")]
fn generate_pdf_response(
    source: &str,
    enabled_tracks: Option<Vec<String>>,
    disabled_lyrics: Option<Vec<String>>,
) -> GeneratePdfResponse {
    let tracks = enabled_tracks.as_deref();
    let lyrics = disabled_lyrics.as_deref();
    match write_pdf_from_source_filtered_with_lyrics(source, "input.jianpu", tracks, lyrics) {
        Ok(pdf) => GeneratePdfResponse::Ok { pdf },
        Err(e) => GeneratePdfResponse::Err {
            diagnostics: vec![diagnostic_from_error(source, e)],
        },
    }
}

#[cfg(feature = "pdf")]
fn generate_split_pdfs_response(source: &str, base_name: &str) -> GenerateSplitPdfsResponse {
    match write_split_pdfs_from_source(source, "input.jianpu", base_name, &[]) {
        Ok(entries) => match zip_split_pdfs(&entries) {
            Ok(zip) => GenerateSplitPdfsResponse::Ok { zip },
            Err(e) => GenerateSplitPdfsResponse::Err {
                diagnostics: vec![diagnostic_from_error(source, e)],
            },
        },
        Err(e) => GenerateSplitPdfsResponse::Err {
            diagnostics: vec![diagnostic_from_error(source, e)],
        },
    }
}

/// Parse and render `.jianpu` source into SVG page strings.
///
/// Always returns a structured value (never throws for parse/render errors):
/// - `{ "status": "ok", "svgs": ["<svg>...</svg>", ...] }`
/// - `{ "status": "err", "diagnostics": [{ "severity": "error", "message": "...",
///   "span": { "start", "end" }, "report": "..." }] }`
///
/// When `enabled_tracks` is omitted, every part is rendered. When provided, only
/// listed abbreviations are kept (`[]` renders no parts).
///
/// When `disabled_lyrics` lists part abbreviations, lyrics are hidden for those parts.
///
/// `span.start` / `span.end` are UTF-8 byte offsets into `source`.
#[wasm_bindgen]
pub fn render(
    source: &str,
    enabled_tracks: Option<Vec<String>>,
    disabled_lyrics: Option<Vec<String>>,
) -> JsValue {
    to_js_value(&render_response(source, enabled_tracks, disabled_lyrics))
}

/// Parse `.jianpu` source and return declared parts from the `[parts]` section.
///
/// - `{ "status": "ok", "parts": [{ "abbreviation", "display_name" }, ...] }`
/// - `{ "status": "err", "diagnostics": [...] }`
#[wasm_bindgen]
pub fn list_parts(source: &str) -> JsValue {
    to_js_value(&list_parts_response(source))
}

/// Find the measure index at a UTF-8 byte offset in the source.
///
/// Returns `{ "status": "ok", "measureIndex": N }` when the offset falls
/// inside a measure's note events, or `{ "status": "notInMeasure" }` otherwise
/// (e.g. when the cursor is in `[metadata]`, `[parts]`, or a directive line).
#[wasm_bindgen]
pub fn get_measure_index_at_offset(source: &str, byte_offset: usize) -> JsValue {
    to_js_value(&get_measure_at_offset_response(source, byte_offset))
}

#[cfg(feature = "wav")]
fn generate_wav_to_js(source: &str, enabled_tracks: Option<Vec<String>>) -> JsValue {
    use js_sys::{Object, Reflect, Uint8Array};

    match generate_wav_response(source, enabled_tracks) {
        GenerateWavResponse::Ok { wav } => {
            let obj = Object::new();
            if Reflect::set(&obj, &JsValue::from_str("status"), &JsValue::from_str("ok")).is_err() {
                return JsValue::from_str("failed to build wav response");
            }
            if Reflect::set(
                &obj,
                &JsValue::from_str("wav"),
                &Uint8Array::from(wav.as_slice()),
            )
            .is_err()
            {
                return JsValue::from_str("failed to attach wav bytes");
            }
            obj.into()
        }
        GenerateWavResponse::Err { diagnostics } => {
            to_js_value(&GenerateWavResponse::Err { diagnostics })
        }
    }
}

/// Parse `.jianpu` source and synthesize WAV audio bytes.
///
/// Available only when the `wav` feature is enabled at build time.
/// Returns the same structured `{ status, ... }` envelope as [`render`]:
/// - `{ "status": "ok", "wav": Uint8Array }`
/// - `{ "status": "err", "diagnostics": [...] }`
#[cfg(feature = "wav")]
#[wasm_bindgen]
pub fn generate_wav(source: &str, enabled_tracks: Option<Vec<String>>) -> JsValue {
    generate_wav_to_js(source, enabled_tracks)
}

/// Synthesize WAV audio for a single measure, with BPM/key context from preceding measures.
///
/// Available only when the `wav` feature is enabled at build time.
/// Returns the same structured envelope as [`generate_wav`]:
/// - `{ "status": "ok", "wav": Uint8Array }`
/// - `{ "status": "err", "diagnostics": [...] }`
#[cfg(feature = "wav")]
#[wasm_bindgen]
pub fn generate_wav_for_measure(
    source: &str,
    measure_index: usize,
    enabled_tracks: Option<Vec<String>>,
) -> JsValue {
    use js_sys::{Object, Reflect, Uint8Array};
    match generate_wav_for_measure_response(source, measure_index, enabled_tracks) {
        GenerateWavResponse::Ok { wav } => {
            let obj = Object::new();
            if Reflect::set(&obj, &JsValue::from_str("status"), &JsValue::from_str("ok")).is_err() {
                return JsValue::from_str("failed to build wav response");
            }
            if Reflect::set(
                &obj,
                &JsValue::from_str("wav"),
                &Uint8Array::from(wav.as_slice()),
            )
            .is_err()
            {
                return JsValue::from_str("failed to attach wav bytes");
            }
            obj.into()
        }
        GenerateWavResponse::Err { diagnostics } => {
            to_js_value(&GenerateWavResponse::Err { diagnostics })
        }
    }
}

#[cfg(feature = "pdf")]
fn generate_pdf_to_js(
    source: &str,
    enabled_tracks: Option<Vec<String>>,
    disabled_lyrics: Option<Vec<String>>,
) -> JsValue {
    use js_sys::{Object, Reflect, Uint8Array};

    match generate_pdf_response(source, enabled_tracks, disabled_lyrics) {
        GeneratePdfResponse::Ok { pdf } => {
            let obj = Object::new();
            if Reflect::set(&obj, &JsValue::from_str("status"), &JsValue::from_str("ok")).is_err() {
                return JsValue::from_str("failed to build pdf response");
            }
            if Reflect::set(
                &obj,
                &JsValue::from_str("pdf"),
                &Uint8Array::from(pdf.as_slice()),
            )
            .is_err()
            {
                return JsValue::from_str("failed to attach pdf bytes");
            }
            obj.into()
        }
        GeneratePdfResponse::Err { diagnostics } => {
            to_js_value(&GeneratePdfResponse::Err { diagnostics })
        }
    }
}

/// Parse `.jianpu` source and write PDF bytes.
///
/// Available only when the `pdf` feature is enabled at build time.
/// Returns the same structured `{ status, ... }` envelope as [`render`]:
/// - `{ "status": "ok", "pdf": Uint8Array }`
/// - `{ "status": "err", "diagnostics": [...] }`
#[cfg(feature = "pdf")]
#[wasm_bindgen]
pub fn generate_pdf(
    source: &str,
    enabled_tracks: Option<Vec<String>>,
    disabled_lyrics: Option<Vec<String>>,
) -> JsValue {
    generate_pdf_to_js(source, enabled_tracks, disabled_lyrics)
}

#[cfg(feature = "pdf")]
fn generate_split_pdfs_to_js(source: &str, base_name: &str) -> JsValue {
    use js_sys::{Object, Reflect, Uint8Array};

    match generate_split_pdfs_response(source, base_name) {
        GenerateSplitPdfsResponse::Ok { zip } => {
            let obj = Object::new();
            if Reflect::set(&obj, &JsValue::from_str("status"), &JsValue::from_str("ok")).is_err() {
                return JsValue::from_str("failed to build split pdf response");
            }
            if Reflect::set(
                &obj,
                &JsValue::from_str("zip"),
                &Uint8Array::from(zip.as_slice()),
            )
            .is_err()
            {
                return JsValue::from_str("failed to attach zip bytes");
            }
            obj.into()
        }
        GenerateSplitPdfsResponse::Err { diagnostics } => {
            to_js_value(&GenerateSplitPdfsResponse::Err { diagnostics })
        }
    }
}

/// Parse `.jianpu` source and write one PDF per part as a ZIP archive.
///
/// Available only when the `pdf` feature is enabled at build time.
/// Returns:
/// - `{ "status": "ok", "zip": Uint8Array }`
/// - `{ "status": "err", "diagnostics": [...] }`
#[cfg(feature = "pdf")]
#[wasm_bindgen]
pub fn generate_split_pdfs(source: &str, base_name: &str) -> JsValue {
    generate_split_pdfs_to_js(source, base_name)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
