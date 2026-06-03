use crate::ast::parsed::JianPuPitch;

#[derive(Debug, Clone)]
pub struct Page {
    pub row_groups: Vec<RowGroup>,
}

/// A horizontal band of the score containing one or more measures.
#[derive(Debug, Clone)]
pub struct RowGroup {
    pub elements: Vec<GridElement>,
    /// Total height in grid rows consumed by this row-group.
    pub height_in_rows: u32,
}

#[derive(Debug, Clone)]
pub struct GridPosition {
    pub column: u32,
    pub row: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct GridElement {
    pub position: GridPosition,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub content: GridContent,
}

#[derive(Debug, Clone)]
pub enum GridContent {
    NoteHead { pitch: JianPuPitch, octave: i8 },
    Rest,
    /// A lyric syllable. `is_cjk` drives font size (CJK = 120% of base).
    Lyric { text: String, is_cjk: bool },
    /// Curved arc connecting two column positions (tie or slur).
    TieOrSlurCurve { from_column: u32, to_column: u32 },
    /// Horizontal underlines below a note head for shorter durations.
    /// count=1 → half beat, count=2 → quarter beat.
    DurationUnderlines { count: u32 },
    BarLine,
}
