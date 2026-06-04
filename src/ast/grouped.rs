use crate::ast::parsed::{JianPuPitch, KeyChange, Syllable};

// ── Public final types ────────────────────────────────────────────────────────

pub struct Metadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    /// Grid cell size in points. Default: 24.
    pub cell_size: u32,
    /// Left margin reserved for part labels in points. Default: 40.
    pub label_width: u32,
}

pub struct Notes {
    pub events: Vec<NoteEvent>,
}

pub struct Lyrics {
    pub syllables: Vec<Syllable>,
}

pub struct PartSlice {
    pub name: Option<String>,
    pub notes: Notes,
    pub lyrics: Option<Lyrics>,
}

pub struct MultiPartMeasure {
    pub time_signature: Option<TimeSignature>,
    pub bpm: Option<u32>,
    pub key: Option<KeyChange>,
    pub parts: Vec<PartSlice>,
}

pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<MultiPartMeasure>,
}

// ── Intermediate grouper types (not part of the public API) ─────────────────

pub(crate) struct GroupedMeasure {
    pub(crate) time_signature: Option<TimeSignature>,
    pub(crate) bpm: Option<u32>,
    pub(crate) key: Option<KeyChange>,
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
