use crate::compositor::types::{DominantBaseline, FontFamily, FontWeight, TextAnchor};

pub struct SvgDocument {
    pub width_pt: f32,
    pub height_pt: f32,
    pub elements: Vec<SvgElement>,
}

pub struct SvgElement {
    pub x: f32,
    pub y: f32,
    pub variant: &'static str,
    pub kind: SvgKind,
}

pub enum SvgKind {
    Text {
        content: String,
        font_size: f32,
        anchor: TextAnchor,
        baseline: DominantBaseline,
        font: FontFamily,
        weight: FontWeight,
        italic: bool,
    },
    Line {
        x2: f32,
        y2: f32,
        stroke_width: f32,
    },
    Circle {
        r: f32,
    },
    Path {
        // Quadratic bezier: x/y from SvgElement; control and end vary
        control_x: f32,
        control_y: f32,
        end_x: f32,
        end_y: f32,
        stroke_width: f32,
    },
}
