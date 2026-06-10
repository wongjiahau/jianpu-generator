use crate::ast::grouped::{GroupedChordNote, NoteEvent, Score};
use crate::ast::parsed::{Extension, JianPuPitch, PartKind, TriadQuality};
use crate::layout::types::{
    GridContent, GridElement, GridPosition, HorizontalAlignment, Page, UnderlineSpan,
    VerticalAlignment,
};

mod layout_engine;

pub mod types;

#[derive(Clone, PartialEq)]
pub(crate) enum SlurKey {
    Pitch(JianPuPitch),
    Chord {
        degree: JianPuPitch,
        triad: TriadQuality,
        extension: Option<Extension>,
        bass_degree: Option<JianPuPitch>,
    },
}

impl SlurKey {
    pub(crate) fn from_chord(chord: &GroupedChordNote) -> Self {
        SlurKey::Chord {
            degree: chord.degree.clone(),
            triad: chord.triad.clone(),
            extension: chord.extension.clone(),
            bass_degree: chord.bass.as_ref().map(|b| b.degree.clone()),
        }
    }
}

struct BeamBufferEntry {
    column: u32,
    underline_count: u32,
    duration: u32,
}

fn flush_beam_buffer(
    buffer: &mut Vec<BeamBufferEntry>,
    row_offset: u32,
    elements: &mut Vec<GridElement>,
) {
    let Some(first) = buffer.first() else {
        return;
    };
    let levels = compute_underline_levels(buffer);
    elements.push(GridElement {
        position: GridPosition {
            column: first.column,
            row: row_offset + 2,
        },
        horizontal_alignment: HorizontalAlignment::Left,
        vertical_alignment: VerticalAlignment::Top,
        content: GridContent::DurationUnderlines { levels },
    });
    buffer.clear();
}

fn compute_underline_levels(buffer: &[BeamBufferEntry]) -> Vec<UnderlineSpan> {
    let (Some(first), Some(last)) = (buffer.first(), buffer.last()) else {
        return Vec::new();
    };
    // Level 1: spans all notes in the group
    let mut levels = vec![UnderlineSpan {
        from_column: first.column,
        to_column: last.column + last.duration,
        last_head_column: last.column,
    }];
    // Level 2+: one span per maximal contiguous sub-run with underline_count >= 2
    let mut run_start: Option<u32> = None;
    let mut run_end: u32 = 0;
    let mut run_last_head: u32 = 0;
    for entry in buffer {
        if entry.underline_count >= 2 {
            if run_start.is_none() {
                run_start = Some(entry.column);
            }
            run_end = entry.column + entry.duration;
            run_last_head = entry.column;
        } else if let Some(start) = run_start.take() {
            levels.push(UnderlineSpan {
                from_column: start,
                to_column: run_end,
                last_head_column: run_last_head,
            });
        }
    }
    if let Some(start) = run_start {
        levels.push(UnderlineSpan {
            from_column: start,
            to_column: run_end,
            last_head_column: run_last_head,
        });
    }
    // Identical level-1 and level-2 spans are intentional: they mean "draw this span twice"
    // (e.g. a lone sixteenth note or a pure-sixteenth beat group must render two underlines).
    levels
}

fn format_chord_symbol(chord: &crate::ast::grouped::GroupedChordNote) -> String {
    use crate::ast::parsed::{Accidental, Extension, JianPuPitch, TriadQuality};

    let degree = match chord.degree {
        JianPuPitch::One => '1',
        JianPuPitch::Two => '2',
        JianPuPitch::Three => '3',
        JianPuPitch::Four => '4',
        JianPuPitch::Five => '5',
        JianPuPitch::Six => '6',
        JianPuPitch::Seven => '7',
    };
    let accidental = match chord.accidental {
        Accidental::Sharp => "♯",
        Accidental::Flat => "♭",
        Accidental::Natural => "",
    };
    let triad = match chord.triad {
        TriadQuality::Major => "",
        TriadQuality::Minor => "m",
        TriadQuality::Diminished => "°",
        TriadQuality::Augmented => "⁺",
    };
    let extension = match &chord.extension {
        Some(Extension::DominantSeventh) => "⁷",
        Some(Extension::MajorSeventh) => "△⁷",
        None => "",
    };
    let mut result = format!("{degree}{accidental}{triad}{extension}");

    if let Some(bass) = &chord.bass {
        let bass_degree = match bass.degree {
            JianPuPitch::One => '1',
            JianPuPitch::Two => '2',
            JianPuPitch::Three => '3',
            JianPuPitch::Four => '4',
            JianPuPitch::Five => '5',
            JianPuPitch::Six => '6',
            JianPuPitch::Seven => '7',
        };
        let bass_acc = match bass.accidental {
            Accidental::Sharp => "♯",
            Accidental::Flat => "♭",
            Accidental::Natural => "",
        };
        result.push('/');
        result.push(bass_degree);
        result.push_str(bass_acc);
    }

    result
}

fn part_row_height(row: &crate::ast::grouped::PartRow) -> u32 {
    use crate::ast::grouped::PartRow;
    match row {
        PartRow::Timed(part) => match part.kind {
            PartKind::Chord => 2,
            PartKind::Notes => 3,
            PartKind::NotesWithLyrics => 4,
        },
        // Ditto rows are not rendered; they occupy no vertical space.
        PartRow::Ditto(_) => 0,
    }
}

fn event_duration(event: &NoteEvent) -> u32 {
    match event {
        NoteEvent::Note(note) => note.duration,
        NoteEvent::Rest(rest) => rest.duration,
        NoteEvent::Chord(chord) => chord.duration,
    }
}

pub(crate) fn measure_beat_width(part: &crate::ast::grouped::PartSlice) -> u32 {
    part.notes.events.iter().map(event_duration).sum()
}

/// Margin on every edge of the page in points (~9 mm).
/// Applied to all four sides: left/right for column fitting, top/bottom for row fitting.
pub(crate) const PAGE_MARGIN: f32 = 25.0;

/// A4 in points: 595 × 842.
/// Row height in points = score.metadata.row_height. Column width varies per row (justified).
pub fn layout(score: &Score, page_width_pt: f32, page_height_pt: f32) -> Vec<Page> {
    layout_engine::LayoutEngine::new(score, page_width_pt, page_height_pt).layout()
}

/// Extend nested tie/slur chains for one note and flush any groups that end here.
pub(crate) fn extend_note_chains(
    chains: &mut Vec<Vec<(u32, SlurKey)>>,
    membership: u8,
    continuation: u8,
    chain_row: u32,
    col: u32,
    key: &SlurKey,
    elements: &mut Vec<GridElement>,
) {
    while chains.len() < membership as usize {
        chains.push(Vec::new());
    }
    for chain in chains.iter_mut().take(membership as usize) {
        chain.push((col, key.clone()));
    }
    for depth in (continuation as usize)..membership as usize {
        if let Some(chain) = chains.get(depth) {
            if chain.len() > 1 {
                flush_chain(chain, chain_row, elements);
            }
        }
        if let Some(chain) = chains.get_mut(depth) {
            chain.clear();
        }
    }
}

/// Emit tie/slur arcs for a completed chain of tied notes (from `(…)` groups).
///
/// Rules:
/// - If the chain contains any pitch change → one **slur** arc from first to last note.
/// - For every consecutive same-pitch pair within the chain → one **tie** arc between them.
fn flush_chain(chain: &[(u32, SlurKey)], chain_row: u32, elements: &mut Vec<GridElement>) {
    if chain.len() <= 1 {
        return;
    }

    let has_key_change = chain
        .windows(2)
        .any(|w| matches!((w.first(), w.get(1)), (Some(a), Some(b)) if a.1 != b.1));

    if has_key_change {
        let (Some(first), Some(last)) = (chain.first(), chain.last()) else {
            return;
        };
        // One slur spanning the entire chain
        elements.push(GridElement {
            position: GridPosition {
                column: first.0,
                row: chain_row,
            },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            content: GridContent::TieOrSlurCurve {
                from_column: first.0,
                to_column: last.0,
            },
        });
    }

    // Tie arc for each consecutive same-pitch pair
    for w in chain.windows(2) {
        let (Some(prev), Some(next)) = (w.first(), w.get(1)) else {
            continue;
        };
        if prev.1 == next.1 {
            elements.push(GridElement {
                position: GridPosition {
                    column: prev.0,
                    row: chain_row,
                },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Top,
                content: GridContent::TieOrSlurCurve {
                    from_column: prev.0,
                    to_column: next.0,
                },
            });
        }
    }
}

fn measure_column_width(measure: &crate::ast::grouped::MultiPartMeasure) -> u32 {
    let max_notes: u32 = measure
        .parts
        .iter()
        .map(|row| measure_beat_width(row.slice()))
        .max()
        .unwrap_or(0);
    max_notes + 1 // +1 for bar line
}

#[cfg(test)]
mod tests;
