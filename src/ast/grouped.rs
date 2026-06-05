use crate::ast::parsed::{JianPuPitch, KeyChange, Syllable};

// ── Public final types ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    /// Row height in points. Controls font sizes, dot radii, and all vertical spacing. Default: 24.
    pub row_height: u32,
    /// Maximum logical columns per row before wrapping. Default: 28.
    pub max_columns: u32,
    /// Left margin reserved for part labels in points. Default: 40.
    pub label_width: u32,
    /// Estimated rendered width of a single digit note number (0–9) in points. Default: 8.
    pub note_number_width: u32,
}

#[derive(Clone)]
pub struct Notes {
    pub events: Vec<NoteEvent>,
}

#[derive(Clone)]
pub struct Lyrics {
    pub syllables: Vec<Syllable>,
}

#[derive(Clone)]
pub struct PartSlice {
    pub name: Option<String>,
    pub notes: Notes,
    pub lyrics: Option<Lyrics>,
}

#[derive(Clone)]
pub struct MultiPartMeasure {
    pub time_signature: Option<TimeSignature>,
    pub bpm: Option<u32>,
    // TODO: key-change rendering (1=X label) is not yet implemented in layout/renderer
    pub key: Option<KeyChange>,
    pub label: Option<String>,
    pub parts: Vec<PartSlice>,
}

#[derive(Clone)]
pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<MultiPartMeasure>,
}

// ── Intermediate grouper types (not part of the public API) ─────────────────

pub(crate) struct GroupedMeasure {
    pub(crate) time_signature: Option<TimeSignature>,
    pub(crate) bpm: Option<u32>,
    pub(crate) key: Option<KeyChange>,
    pub(crate) label: Option<String>,
    pub(crate) notes: Notes,
}

pub(crate) struct GroupedPart {
    pub(crate) name: Option<String>,
    pub(crate) measures: Vec<GroupedMeasure>,
    /// Flat lyrics list. `None` means no [lyrics] section was provided.
    pub(crate) lyrics: Option<Vec<Syllable>>,
}

// ── Shared note types ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

#[derive(Clone)]
pub enum NoteEvent {
    Note(GroupedNote),
    Rest(GroupedRest),
}

#[derive(Clone)]
pub struct GroupedNote {
    pub pitch: JianPuPitch,
    pub octave: i8,
    /// Duration in quarter-beats, including any beats added by `-` extensions.
    pub duration: u32,
    /// True if this note is tied/slurred to the next note.
    pub tie: bool,
}

#[derive(Clone)]
pub struct GroupedRest {
    /// Duration in quarter-beats, including any beats added by `-` extensions.
    pub duration: u32,
}
