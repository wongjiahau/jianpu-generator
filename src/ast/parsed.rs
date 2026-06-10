use crate::error::Spanned;

#[derive(Debug)]
pub struct ParsedScore {
    pub events: Vec<Spanned<ScoreEvent>>,
}

#[derive(Debug)]
pub struct ParsedLyrics {
    pub syllables: Vec<Syllable>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartDecl {
    pub abbreviation: String,
    pub display_name: String,
    pub kind: PartKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PartKind {
    Chord,
    Notes,
    NotesWithLyrics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreLineRole {
    Chord,
    Notes,
    Lyrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoreLineSlot {
    pub track_index: usize,
    pub role: ScoreLineRole,
}

impl PartDecl {
    pub fn score_line_roles(&self) -> &'static [ScoreLineRole] {
        match self.kind {
            PartKind::Chord => &[ScoreLineRole::Chord],
            PartKind::Notes => &[ScoreLineRole::Notes],
            PartKind::NotesWithLyrics => &[ScoreLineRole::Notes, ScoreLineRole::Lyrics],
        }
    }
}

pub fn flatten_score_line_slots(declarations: &[PartDecl]) -> Vec<ScoreLineSlot> {
    let mut slots = Vec::new();
    for (track_index, decl) in declarations.iter().enumerate() {
        for &role in decl.score_line_roles() {
            slots.push(ScoreLineSlot { track_index, role });
        }
    }
    slots
}

#[derive(Debug)]
pub enum ParsedTrack {
    Timed(ParsedTimedTrack),
}

#[derive(Debug)]
pub struct ParsedTimedTrack {
    pub abbreviation: String,
    pub display_name: String,
    pub kind: PartKind,
    pub score: ParsedScore,
    pub lyrics: Option<ParsedLyrics>,
}

#[derive(Debug)]
pub struct ParsedDocument {
    #[allow(dead_code)]
    pub filename: String,
    pub metadata: ParsedMetadata,
    #[allow(dead_code)] // reserved for future legend rendering
    pub declarations: Vec<PartDecl>,
    pub tracks: Vec<ParsedTrack>,
    pub directive_events_per_measure: Vec<Vec<Spanned<ScoreEvent>>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum TriadQuality {
    Major,
    Minor,
    Augmented,
    Diminished,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Extension {
    DominantSeventh,
    MajorSeventh,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BassDegree {
    pub degree: JianPuPitch,
    pub accidental: Accidental,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedChordSymbol {
    pub degree: JianPuPitch,
    pub accidental: Accidental,
    pub triad: TriadQuality,
    pub extension: Option<Extension>,
    pub bass: Option<BassDegree>,
}

#[derive(Debug)]
pub struct ParsedMetadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    pub row_height: Option<u32>,
    pub max_columns: Option<u32>,
    pub label_width: Option<u32>,
    pub note_number_width: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScoreEvent {
    Note(ParsedNote),
    Chord(ParsedChordNote),
    Rest(ParsedRest),
    BpmChange(u32),
    KeyChange(KeyChange),
    TimeSignatureChange {
        numerator: u8,
        denominator: u8,
    },
    /// Internal or explicit padding: extends the previous note by one full beat (4 quarter-beats).
    Extension,
    /// Legacy tie marker retained for lyric-slot counting paths; use `(…)` groups in input.
    TieMarker,
    LabelChange(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedNote {
    pub pitch: JianPuPitch,
    /// Octave offset from the default octave. 0 = default, positive = up, negative = down.
    pub octave: i8,
    /// Duration in quarter-beats. For dotted notes this already includes the added half-value.
    pub duration: u32,
    /// Whether this note is tied/slurred to the next note (from a `(…)` group).
    pub tie: bool,
    /// Number of nested `(…)` groups this note belongs to.
    pub group_membership: u8,
    /// Number of those groups that continue past this note.
    pub group_continuation: u8,
    /// Whether `.` was present as a dotted-note suffix.
    pub dotted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedChordNote {
    pub degree: JianPuPitch,
    pub accidental: Accidental,
    pub triad: TriadQuality,
    pub extension: Option<Extension>,
    pub bass: Option<BassDegree>,
    pub duration: u32,
    pub tie: bool,
    pub group_membership: u8,
    pub group_continuation: u8,
    pub dotted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRest {
    /// Duration in quarter-beats. For dotted rests this already includes the added half-value.
    pub duration: u32,
    /// Whether `.` was present as a dotted-rest suffix.
    pub dotted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JianPuPitch {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyChange {
    pub note: Note,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub name: NoteName,
    pub octave: u8,
    pub accidental: Accidental,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoteName {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Accidental {
    Flat,
    Sharp,
    Natural,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Syllable {
    pub text: String,
    /// True if `-` follows this syllable in the lyrics section.
    pub held: bool,
}
