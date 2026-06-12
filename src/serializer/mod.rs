use crate::compositor::types::{DominantBaseline, FontFamily, FontWeight, TextAnchor};
use crate::renderer::new_types::{SvgDocument, SvgElement, SvgKind};

pub fn serialize(documents: &[SvgDocument]) -> Vec<String> {
    documents.iter().map(serialize_doc).collect()
}

fn serialize_doc(doc: &SvgDocument) -> String {
    let mut body = String::new();
    for el in &doc.elements {
        serialize_element(el, &mut body);
    }
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 {:.0} {:.0}">{}</svg>"#,
        doc.width_pt, doc.height_pt, body
    )
}

fn serialize_element(el: &SvgElement, out: &mut String) {
    match &el.kind {
        SvgKind::Text {
            content,
            font_size,
            anchor,
            baseline,
            font,
            weight,
            italic,
        } => {
            let anchor_str = match anchor {
                TextAnchor::Start => "start",
                TextAnchor::Middle => "middle",
                TextAnchor::End => "end",
            };
            let baseline_str = match baseline {
                DominantBaseline::Middle => "middle",
                DominantBaseline::Hanging => "hanging",
                DominantBaseline::Ideographic => "ideographic",
            };
            let font_str = match font {
                FontFamily::Monospace => "monospace",
                FontFamily::SansSerif => "sans-serif",
            };
            let weight_str = match weight {
                FontWeight::Normal => "normal",
                FontWeight::Bold => "bold",
            };
            let style_str = if *italic {
                "font-style=\"italic\" ".to_string()
            } else {
                String::new()
            };
            out.push_str(&format!(
                r#"<text x="{:.1}" y="{:.1}" data-variant="{}" font-size="{:.1}" text-anchor="{}" dominant-baseline="{}" font-family="{}" font-weight="{}" {}>{}</text>"#,
                el.x, el.y, el.variant, font_size, anchor_str, baseline_str, font_str, weight_str, style_str,
                escape_xml(content)
            ));
        }
        SvgKind::Line {
            x2,
            y2,
            stroke_width,
        } => {
            out.push_str(&format!(
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" data-variant="{}" stroke="black" stroke-width="{:.1}"/>"#,
                el.x, el.y, x2, y2, el.variant, stroke_width
            ));
        }
        SvgKind::Circle { r } => {
            out.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" data-variant="{}" r="{:.1}" fill="black"/>"#,
                el.x, el.y, el.variant, r
            ));
        }
        SvgKind::Path {
            control_x,
            control_y,
            end_x,
            end_y,
            stroke_width,
        } => {
            out.push_str(&format!(
                r#"<path d="M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}" data-variant="{}" fill="none" stroke="black" stroke-width="{:.1}"/>"#,
                el.x, el.y, control_x, control_y, end_x, end_y, el.variant, stroke_width
            ));
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::types::{DominantBaseline, FontFamily, FontWeight, TextAnchor};
    use crate::renderer::new_types::{SvgDocument, SvgElement, SvgKind};

    fn text_doc(content: &str) -> SvgDocument {
        SvgDocument {
            width_pt: 595.0,
            height_pt: 842.0,
            elements: vec![SvgElement {
                x: 10.0,
                y: 20.0,
                variant: "text",
                kind: SvgKind::Text {
                    content: content.to_string(),
                    font_size: 12.0,
                    anchor: TextAnchor::Middle,
                    baseline: DominantBaseline::Middle,
                    font: FontFamily::SansSerif,
                    weight: FontWeight::Normal,
                    italic: false,
                },
            }],
        }
    }

    #[test]
    fn produces_valid_svg_wrapper() {
        let result = serialize(&[text_doc("hello")]);
        assert_eq!(result.len(), 1);
        assert!(result[0].starts_with("<svg"), "should start with <svg");
        assert!(result[0].ends_with("</svg>"), "should end with </svg>");
    }

    #[test]
    fn xml_special_chars_are_escaped() {
        let result = serialize(&[text_doc("<b>&\"</b>")]);
        assert!(result[0].contains("&lt;b&gt;&amp;&quot;&lt;/b&gt;"));
    }

    #[test]
    fn circle_serializes_correctly() {
        let doc = SvgDocument {
            width_pt: 100.0,
            height_pt: 100.0,
            elements: vec![SvgElement {
                x: 5.0,
                y: 5.0,
                variant: "note-head",
                kind: SvgKind::Circle { r: 3.0 },
            }],
        };
        let result = serialize(&[doc]);
        assert!(result[0].contains("<circle"), "should contain circle");
        assert!(result[0].contains(r#"r="3.0""#));
    }

    #[test]
    fn line_serializes_correctly() {
        let doc = SvgDocument {
            width_pt: 100.0,
            height_pt: 100.0,
            elements: vec![SvgElement {
                x: 0.0,
                y: 0.0,
                variant: "bar-line",
                kind: SvgKind::Line {
                    x2: 50.0,
                    y2: 0.0,
                    stroke_width: 1.0,
                },
            }],
        };
        let result = serialize(&[doc]);
        assert!(result[0].contains("<line"), "should contain line");
    }

    #[test]
    fn path_serializes_correctly() {
        let doc = SvgDocument {
            width_pt: 100.0,
            height_pt: 100.0,
            elements: vec![SvgElement {
                x: 0.0,
                y: 0.0,
                variant: "tie-or-slur",
                kind: SvgKind::Path {
                    control_x: 25.0,
                    control_y: -10.0,
                    end_x: 50.0,
                    end_y: 0.0,
                    stroke_width: 1.5,
                },
            }],
        };
        let result = serialize(&[doc]);
        assert!(result[0].contains("<path"), "should contain path");
        assert!(result[0].contains("fill=\"none\""));
    }

    #[test]
    fn text_element_has_data_variant() {
        let result = serialize(&[text_doc("hello")]);
        assert!(result[0].contains(r#"data-variant="text""#));
    }

    #[test]
    fn circle_element_has_data_variant() {
        let doc = SvgDocument {
            width_pt: 100.0,
            height_pt: 100.0,
            elements: vec![SvgElement {
                x: 5.0,
                y: 5.0,
                variant: "note-head",
                kind: SvgKind::Circle { r: 3.0 },
            }],
        };
        let result = serialize(&[doc]);
        assert!(result[0].contains(r#"data-variant="note-head""#));
    }

    #[test]
    fn line_element_has_data_variant() {
        let doc = SvgDocument {
            width_pt: 100.0,
            height_pt: 100.0,
            elements: vec![SvgElement {
                x: 0.0,
                y: 0.0,
                variant: "bar-line",
                kind: SvgKind::Line {
                    x2: 50.0,
                    y2: 0.0,
                    stroke_width: 1.0,
                },
            }],
        };
        let result = serialize(&[doc]);
        assert!(result[0].contains(r#"data-variant="bar-line""#));
    }

    #[test]
    fn path_element_has_data_variant() {
        let doc = SvgDocument {
            width_pt: 100.0,
            height_pt: 100.0,
            elements: vec![SvgElement {
                x: 0.0,
                y: 0.0,
                variant: "tie-or-slur",
                kind: SvgKind::Path {
                    control_x: 25.0,
                    control_y: -10.0,
                    end_x: 50.0,
                    end_y: 0.0,
                    stroke_width: 1.5,
                },
            }],
        };
        let result = serialize(&[doc]);
        assert!(result[0].contains(r#"data-variant="tie-or-slur""#));
    }
}
