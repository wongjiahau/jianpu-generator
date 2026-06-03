use crate::ast::parsed::{JianPuPitch, KeyChange, Syllable};

pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<Measure>,
    pub lyrics: Vec<Syllable>,
}

pub struct Metadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    /// Grid cell size in points. Default: 24.
    pub cell_size: u32,
}

pub struct Measure {
    pub time_signature: TimeSignature,
    pub bpm: u32,
    pub key: KeyChange,
    pub notes: Vec<NoteEvent>,
}

pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

pub enum NoteEvent {
    Note(GroupedNote),
    Rest(GroupedRest),
}

pub struct GroupedNote {
    pub pitch: JianPuPitch,
    pub octave: i8,
    /// Duration in quarter-beats, including any beats added by `-` extensions.
    pub duration: u32,
    /// True if this note is tied/slurred to the next note.
    pub tie: bool,
}

pub struct GroupedRest {
    /// Duration in quarter-beats, including any beats added by `-` extensions.
    pub duration: u32,
}
