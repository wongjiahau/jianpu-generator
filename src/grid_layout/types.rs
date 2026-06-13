use crate::ast::parsed::JianPuPitch;

#[derive(Debug, Clone)]
pub struct GridPage {
    pub width_pt: f32,
    pub height_pt: f32,
    pub rows: Vec<GridRow>,
}

#[derive(Debug, Clone)]
pub struct GridRow {
    pub height_pt: f32,
    pub column_count: u32,
    pub elements: Vec<GridElement>,
}

impl GridRow {
    /// Column width in points, given the usable page width.
    pub fn column_width_pt(&self, usable_width_pt: f32) -> f32 {
        usable_width_pt / self.column_count as f32
    }
}

#[derive(Debug, Clone)]
pub struct GridElement {
    pub column: u32,
    pub column_span: u32,
    pub halign: HAlign,
    pub valign: VAlign,
    pub content: GridContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone)]
pub enum GridContent {
    /// Note head. `octave > 0` = dots above, `octave < 0` = dots below,
    /// `octave.abs()` = dot count. Octave rendered inline by the renderer;
    /// OctaveDot sub-rows exist for vertical spacing only.
    NoteHead {
        pitch: JianPuPitch,
        octave: i8,
        dotted: bool,
    },
    Rest {
        dotted: bool,
    },
    NoteDash,
    /// Spacing-only row for octave dots. Resolver emits nothing for this.
    OctaveDot,
    ChordSymbol(String),
    /// Durational underline. `level=0` half-beat, `level=1` quarter-beat.
    Underline {
        level: u32,
    },
    /// Same-system tie/slur arc: from center of from-column to center of to-column.
    TieOrSlur,
    /// Cross-system arc, first system: center of from-column to right edge of system.
    TieOrSlurTail,
    /// Cross-system arc, last system: left edge of system to center of to-column.
    TieOrSlurHead,
    /// Vertical bar line. `height_pt` baked in by grid layout layer.
    BarLine {
        height_pt: f32,
    },
    /// Full-width horizontal system separator.
    HorizontalLine,
    /// Part name at column=0, column_span=4 in the note-head sub-row.
    RowLabel(String),
    LyricSyllable(String),
    Bpm(u32),
    TimeSignature {
        numerator: u32,
        denominator: u32,
    },
    SectionLabel(String),
    BarNumber(u32),
    /// Generic styled text for header and footer rows.
    Text {
        content: String,
        font_size: f32,
        bold: bool,
        italic: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Header {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
}
