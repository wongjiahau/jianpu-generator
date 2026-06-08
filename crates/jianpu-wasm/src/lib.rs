use jianpu_generator::{error::JianPuError, error_reporter, render_svgs_from_source};
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
#[serde(tag = "status", rename_all = "camelCase")]
enum RenderResponse {
    Ok { svgs: Vec<String> },
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

fn render_response(source: &str) -> RenderResponse {
    match render_svgs_from_source(source, "input.jianpu") {
        Ok(svgs) => RenderResponse::Ok { svgs },
        Err(e) => RenderResponse::Err {
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
/// `span.start` / `span.end` are UTF-8 byte offsets into `source`.
#[wasm_bindgen]
pub fn render(source: &str) -> JsValue {
    to_js_value(&render_response(source))
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
        let resp = render_response(input);
        match resp {
            RenderResponse::Ok { svgs } => {
                assert_eq!(svgs.len(), 1);
                assert!(svgs[0].starts_with("<svg"));
            }
            RenderResponse::Err { .. } => panic!("expected ok"),
        }
    }

    #[test]
    fn err_response_has_structured_diagnostic() {
        let resp = render_response("not valid jianpu");
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
        let resp = render_response(source);
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
        let resp = render_response(source);
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
