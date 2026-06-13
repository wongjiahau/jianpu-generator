use jianpu_generator::{error::JianPuError, error_reporter};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SpanOut {
    /// UTF-8 byte offset (inclusive).
    pub(crate) start: usize,
    /// UTF-8 byte offset (exclusive).
    pub(crate) end: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub(crate) enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DiagnosticOut {
    pub(crate) severity: DiagnosticSeverity,
    pub(crate) message: String,
    pub(crate) span: SpanOut,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) report: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct PartOut {
    pub(crate) abbreviation: String,
    pub(crate) display_name: String,
    pub(crate) has_lyrics: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum RenderResponse {
    Ok { svgs: Vec<String> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum ListPartsResponse {
    Ok { parts: Vec<PartOut> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum MeasureAtOffsetResponse {
    Ok { measure_index: usize },
    NotInMeasure,
}

#[cfg(feature = "wav")]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum GenerateWavResponse {
    Ok { wav: Vec<u8> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

#[cfg(feature = "pdf")]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum GeneratePdfResponse {
    Ok { pdf: Vec<u8> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

#[cfg(feature = "pdf")]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum GenerateSplitPdfsResponse {
    Ok { zip: Vec<u8> },
    Err { diagnostics: Vec<DiagnosticOut> },
}

pub(crate) fn diagnostic_from_error(source: &str, e: JianPuError) -> DiagnosticOut {
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

pub(crate) fn to_js_value<T: Serialize>(value: &T) -> JsValue {
    match serde_wasm_bindgen::to_value(value) {
        Ok(v) => v,
        Err(err) => JsValue::from_str(&format!("serialization failed: {err}")),
    }
}
