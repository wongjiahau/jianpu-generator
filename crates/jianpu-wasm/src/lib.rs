use jianpu_generator::{
    error::JianPuError, error_reporter, list_parts_from_source, render_svgs_from_source_filtered,
};
#[cfg(feature = "pdf")]
use jianpu_generator::write_pdf_from_source_filtered;
#[cfg(feature = "wav")]
use jianpu_generator::write_wav_from_source_filtered;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SpanOut {
    /// UTF-8 byte offset (inclusive).
    start: usize,
    /// UTF-8 byte offset (exclusive).
    end: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct DiagnosticOut {
    severity: DiagnosticSeverity,
    message: String,
    span: SpanOut,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PartOut {
    abbreviation: String,
    display_name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
enum RenderResponse {
    Ok { svgs: Vec<String> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
enum ListPartsResponse {
    Ok { parts: Vec<PartOut> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

#[cfg(feature = "wav")]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
enum GenerateWavResponse {
    Ok { wav: Vec<u8> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

#[cfg(feature = "pdf")]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
enum GeneratePdfResponse {
    Ok { pdf: Vec<u8> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

fn diagnostic_from_error(source: &str, e: JianPuError) -> DiagnosticOut {
    let report = error_reporter::render_with_source(source, &e);
    DiagnosticOut {
        severity: DiagnosticSeverity::Error,
        message: e.message,
        span: SpanOut {
            start: e.span.start,
            end: e.span.end,
        },
        report: Some(report),
    }
}

fn render_response(source: &str, enabled_tracks: Option<Vec<String>>) -> RenderResponse {
    let tracks = enabled_tracks.as_deref();
    match render_svgs_from_source_filtered(source, "input.jianpu", tracks) {
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
                })
                .collect(),
        },
        Err(e) => ListPartsResponse::Err {
            diagnostics: vec![diagnostic_from_error(source, e)],
        },
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

#[cfg(feature = "pdf")]
fn generate_pdf_response(source: &str, enabled_tracks: Option<Vec<String>>) -> GeneratePdfResponse {
    let tracks = enabled_tracks.as_deref();
    match write_pdf_from_source_filtered(source, "input.jianpu", tracks) {
        Ok(pdf) => GeneratePdfResponse::Ok { pdf },
        Err(e) => GeneratePdfResponse::Err {
            diagnostics: vec![diagnostic_from_error(source, e)],
        },
    }
}

fn to_js_value<T: Serialize>(value: &T) -> JsValue {
    match serde_wasm_bindgen::to_value(value) {
        Ok(v) => v,
        Err(err) => JsValue::from_str(&format!("serialization failed: {err}")),
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
/// `span.start` / `span.end` are UTF-8 byte offsets into `source`.
#[wasm_bindgen]
pub fn render(source: &str, enabled_tracks: Option<Vec<String>>) -> JsValue {
    to_js_value(&render_response(source, enabled_tracks))
}

/// Parse `.jianpu` source and return declared parts from the `[parts]` section.
///
/// - `{ "status": "ok", "parts": [{ "abbreviation", "display_name" }, ...] }`
/// - `{ "status": "err", "diagnostics": [...] }`
#[wasm_bindgen]
pub fn list_parts(source: &str) -> JsValue {
    to_js_value(&list_parts_response(source))
}

#[cfg(feature = "wav")]
fn generate_wav_to_js(source: &str, enabled_tracks: Option<Vec<String>>) -> JsValue {
    use js_sys::{Object, Reflect, Uint8Array};

    match generate_wav_response(source, enabled_tracks) {
        GenerateWavResponse::Ok { wav } => {
            let obj = Object::new();
            if Reflect::set(
                &obj,
                &JsValue::from_str("status"),
                &JsValue::from_str("ok"),
            )
            .is_err()
            {
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

#[cfg(feature = "pdf")]
fn generate_pdf_to_js(source: &str, enabled_tracks: Option<Vec<String>>) -> JsValue {
    use js_sys::{Object, Reflect, Uint8Array};

    match generate_pdf_response(source, enabled_tracks) {
        GeneratePdfResponse::Ok { pdf } => {
            let obj = Object::new();
            if Reflect::set(
                &obj,
                &JsValue::from_str("status"),
                &JsValue::from_str("ok"),
            )
            .is_err()
            {
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
pub fn generate_pdf(source: &str, enabled_tracks: Option<Vec<String>>) -> JsValue {
    generate_pdf_to_js(source, enabled_tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_response_has_svgs() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "Melody = notes lyrics\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "a b c d\n",
        );
        let resp = render_response(input, None);
        match resp {
            RenderResponse::Ok { svgs } => {
                assert_eq!(svgs.len(), 1);
                assert!(svgs[0].starts_with("<svg"));
            }
            RenderResponse::Err { .. } => panic!("expected ok"),
        }
    }

    #[test]
    fn list_parts_response_returns_declarations() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "Soprano = notes\n",
            "Alto = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
        );
        let resp = list_parts_response(input);
        match resp {
            ListPartsResponse::Ok { parts } => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].abbreviation, "Soprano");
                assert_eq!(parts[1].abbreviation, "Alto");
            }
            ListPartsResponse::Err { diagnostics } => {
                panic!("expected ok: {}", diagnostics[0].message);
            }
        }
    }

    #[test]
    fn render_with_enabled_tracks_filters_parts() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "Soprano = notes\n",
            "Alto = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
        );
        let all = match render_response(input, None) {
            RenderResponse::Ok { svgs } => svgs,
            RenderResponse::Err { .. } => panic!("expected ok"),
        };
        let soprano_only = match render_response(input, Some(vec!["Soprano".into()])) {
            RenderResponse::Ok { svgs } => svgs,
            RenderResponse::Err { .. } => panic!("expected ok"),
        };
        assert_ne!(all[0], soprano_only[0]);
    }

    #[test]
    fn err_response_has_structured_diagnostic() {
        let resp = render_response("not valid jianpu", None);
        match resp {
            RenderResponse::Err { diagnostics } => {
                assert!(!diagnostics.is_empty());
                let d = &diagnostics[0];
                assert_eq!(d.severity, DiagnosticSeverity::Error);
                assert!(!d.message.is_empty());
                assert!(d.report.as_ref().is_some_and(|r| !r.is_empty()));
            }
            RenderResponse::Ok { .. } => panic!("expected err"),
        }
    }

    #[test]
    fn demo_jianpu_renders() {
        let source = include_str!("../../../demo.jianpu");
        let resp = render_response(source, None);
        match resp {
            RenderResponse::Ok { svgs } => {
                assert!(
                    !svgs.is_empty(),
                    "demo.jianpu should render in the wasm path used by the web editor"
                );
            }
            RenderResponse::Err { diagnostics } => {
                panic!(
                    "demo.jianpu failed in wasm render path: {}",
                    diagnostics[0].message
                );
            }
        }
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn demo_jianpu_generates_pdf() {
        let source = include_str!("../../../demo.jianpu");
        let resp = generate_pdf_response(source, None);
        match resp {
            GeneratePdfResponse::Ok { pdf } => {
                assert!(pdf.len() > 4);
                assert_eq!(&pdf[0..4], b"%PDF");
            }
            GeneratePdfResponse::Err { diagnostics } => {
                panic!(
                    "demo.jianpu failed in wasm pdf path: {}",
                    diagnostics[0].message
                );
            }
        }
    }

    #[cfg(feature = "wav")]
    #[test]
    fn demo_jianpu_generates_wav() {
        let source = include_str!("../../../demo.jianpu");
        let resp = generate_wav_response(source, None);
        match resp {
            GenerateWavResponse::Ok { wav } => {
                assert!(wav.len() > 4);
                assert_eq!(&wav[0..4], b"RIFF");
            }
            GenerateWavResponse::Err { diagnostics } => {
                panic!(
                    "demo.jianpu failed in wasm wav path: {}",
                    diagnostics[0].message
                );
            }
        }
    }

    #[test]
    fn err_span_is_utf8_byte_offset() {
        let source = concat!(
            "[metadata]\n",
            "title = \"你好\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "Melody = notes lyrics\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 x 4\n",
            "a b c d\n",
        );
        let token_byte_start = source.find('x').expect("error token in source");
        let resp = render_response(source, None);
        let RenderResponse::Err { diagnostics } = resp else {
            panic!("expected err");
        };
        assert_eq!(diagnostics[0].span.start, token_byte_start);
        assert!(
            token_byte_start > 4,
            "span is absolute in source, not line-local"
        );
    }
}
