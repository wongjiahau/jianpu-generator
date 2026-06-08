use crate::ast::grouped::{NoteEvent, Score};
use crate::ast::parsed::JianPuPitch;
use crate::layout::types::{
    GridContent, GridElement, GridPosition, HorizontalAlignment, Page, UnderlineSpan,
    VerticalAlignment,
};

mod layout_engine;

pub mod types;

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

fn format_chord_symbol(chord: &crate::ast::grouped::GroupedChord) -> String {
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
        PartRow::Notes(part) => {
            if part.lyrics.is_some() {
                4
            } else {
                3
            }
        }
        PartRow::Chord(_) => 2,
    }
}

fn compute_prefix_width(measure: &crate::ast::grouped::MultiPartMeasure) -> u32 {
    let mut width = 0;
    if measure.time_signature.is_some() {
        width += 2;
    }
    if measure.bpm.is_some() {
        width += 2;
    }
    width
}

/// Margin on every edge of the page in points (~9 mm).
/// Applied to all four sides: left/right for column fitting, top/bottom for row fitting.
pub(crate) const PAGE_MARGIN: f32 = 25.0;

/// A4 in points: 595 × 842.
/// Row height in points = score.metadata.row_height. Column width varies per row (justified).
pub fn layout(score: &Score, page_width_pt: f32, page_height_pt: f32) -> Vec<Page> {
    layout_engine::LayoutEngine::new(score, page_width_pt, page_height_pt).layout()
}

/// Emit tie/slur arcs for a completed chain of tied notes (from `(…)` groups).
///
/// Rules:
/// - If the chain contains any pitch change → one **slur** arc from first to last note.
/// - For every consecutive same-pitch pair within the chain → one **tie** arc between them.
fn flush_chain(chain: &[(u32, JianPuPitch)], chain_row: u32, elements: &mut Vec<GridElement>) {
    if chain.len() <= 1 {
        return;
    }

    let has_pitch_change = chain
        .windows(2)
        .any(|w| matches!((w.first(), w.get(1)), (Some(a), Some(b)) if a.1 != b.1));

    if has_pitch_change {
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
    use crate::ast::grouped::PartRow;
    let max_notes: u32 = measure
        .parts
        .iter()
        .filter_map(|row| {
            if let PartRow::Notes(p) = row {
                Some(p)
            } else {
                None
            }
        })
        .map(|part| {
            part.notes
                .events
                .iter()
                .map(|n| match n {
                    NoteEvent::Note(note) => note.duration,
                    NoteEvent::Rest(rest) => rest.duration,
                })
                .sum::<u32>()
        })
        .max()
        .unwrap_or(0);
    max_notes + 1 // +1 for bar line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grouper;
    use crate::parser;

    fn syllables_to_line(syllables: &[crate::ast::parsed::Syllable]) -> String {
        syllables
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Build a single-part score with lyrics from bar-separated notes (use `|` to separate bars).
    /// `lyrics_str` syllables are allocated per bar to match each bar's lyric-slot count.
    fn make_score(score_str: &str, lyrics_str: &str) -> Score {
        use crate::parser::score::{token_parser, tokenizer};
        use crate::utils::{count_lyric_slots_in_events, LyricTieState};

        let bars: Vec<&str> = score_str
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let all_syllables = crate::utils::tokenize_lyrics(lyrics_str);
        let mut syllable_idx = 0usize;
        let mut tie_state = LyricTieState::default();
        let mut score_content = String::new();
        score_content.push_str("(time=4/4 key=C4 bpm=120)\n");
        for bar in bars {
            score_content.push_str(bar);
            score_content.push('\n');
            let tokens = tokenizer::tokenize(bar, 0);
            let events = token_parser::parse_tokens(tokens, &mut token_parser::GroupParseState::default())
                .expect("test score tokens");
            let slots = count_lyric_slots_in_events(&events, &mut tie_state) as usize;
            if slots == 0 {
                score_content.push_str("_\n");
            } else {
                let end = (syllable_idx + slots).min(all_syllables.len());
                let bar_syllables = &all_syllables[syllable_idx..end];
                assert_eq!(
                    bar_syllables.len(),
                    slots,
                    "make_score: not enough lyrics for bar {bar:?} (need {slots})"
                );
                syllable_idx = end;
                score_content.push_str(&syllables_to_line(bar_syllables));
                score_content.push('\n');
            }
            score_content.push('\n'); // blank line separating bar groups
        }
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n{score_content}"
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap()
    }

    /// Build a score from a pre-formatted score_content string (new interleaved format).
    /// score_section must be the full content after `[score]\n` in new interleaved syntax.
    fn make_score_raw(score_section: &str, lyrics_str: &str) -> Score {
        // score_section is already in new interleaved format passed by callers.
        // lyrics_str is ignored here as it's embedded in score_section.
        let _ = lyrics_str; // lyrics are inline in score_section
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n{score_section}"
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap()
    }

    #[test]
    fn first_measure_emits_time_signature_label_at_column_zero() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].position.column, 3);
        if let GridContent::TimeSignatureLabel {
            numerator,
            denominator,
        } = &labels[0].content
        {
            assert_eq!(*numerator, 4);
            assert_eq!(*denominator, 4);
        } else {
            panic!("expected TimeSignatureLabel");
        }
    }

    #[test]
    fn first_measure_emits_bpm_label_at_column_two() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].position.column, 5);
        if let GridContent::BpmLabel { bpm } = &labels[0].content {
            assert_eq!(*bpm, 120);
        } else {
            panic!("expected BpmLabel");
        }
    }

    #[test]
    fn note_heads_start_after_both_label_columns() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let note_heads: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
            .collect();
        assert_eq!(note_heads[0].position.column, 7);
    }

    #[test]
    fn unchanged_time_signature_emits_no_second_label() {
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1, "only one time signature label expected for two measures with identical time signature on the same line");
    }

    #[test]
    fn unchanged_bpm_emits_no_second_label() {
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(
            labels.len(),
            1,
            "only one BPM label expected for two measures with identical BPM on the same line"
        );
    }

    #[test]
    fn time_signature_change_emits_second_label() {
        // Two bars: first 4/4 (4 quarter notes), second 3/4 (3 quarter notes), each with lyrics.
        let score = make_score_raw(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(time=3/4)\n1 2 3\ne f g\n",
            "",
        );
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(
            labels.len(),
            2,
            "expected one label per distinct time signature"
        );
    }

    #[test]
    fn bpm_change_emits_second_label() {
        // Two bars: first at bpm=120, second at bpm=90, each with lyrics.
        let score = make_score_raw(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(bpm=90)\n5 6 7 1\ne f g h\n",
            "",
        );
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(
            labels.len(),
            2,
            "expected one BPM label per distinct BPM value"
        );
    }

    const A4_WIDTH: f32 = 595.0; // points
    const A4_HEIGHT: f32 = 842.0; // points

    #[test]
    fn header_is_populated_on_every_page() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert!(!pages.is_empty());
        for page in &pages {
            assert_eq!(page.header.title, "t");
            assert_eq!(page.header.author, "a");
            assert_eq!(page.header.subtitle, None);
        }
    }

    #[test]
    fn footer_page_numbers_are_correct() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let total = pages.len() as u32;
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(page.footer.page, i as u32 + 1);
            assert_eq!(page.footer.total, total);
        }
    }

    #[test]
    fn produces_at_least_one_page() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert!(!pages.is_empty());
    }

    #[test]
    fn note_heads_are_present() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let note_heads: Vec<_> = all_elements
            .iter()
            .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
            .collect();
        assert_eq!(note_heads.len(), 4);
    }

    #[test]
    fn lyrics_are_present() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let lyrics: Vec<_> = all_elements
            .iter()
            .filter(|e| matches!(e.content, GridContent::Lyric { .. }))
            .collect();
        assert_eq!(lyrics.len(), 4);
    }

    #[test]
    fn two_different_notes_emit_one_slur() {
        // 1~ 2: different pitches → one slur from col 5 to col 9
        let score = make_score("(12) 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (7, 11));
    }

    #[test]
    fn three_note_slur_emits_one_curve() {
        // 3~2~1: all different pitches → one slur from col 5 to col 13
        let score = make_score("(321) 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (7, 15));
    }

    #[test]
    fn mixed_chain_emits_slur_and_tie() {
        // (433) 2: chain [4@5, 3@9, 3@13]
        // → one slur from 5 to 13 (pitch change exists)
        // → one tie from 9 to 13 (same-pitch pair 3~3)
        let score = make_score("(433) 2", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let mut curves = collect_curves(&pages);
        curves.sort();
        assert_eq!(curves.len(), 2);
        assert_eq!(curves[0], (7, 15)); // slur
        assert_eq!(curves[1], (11, 15)); // tie
    }

    #[test]
    fn same_pitch_chain_emits_only_tie() {
        // (11) 2 3: same pitches → one tie, no slur
        let score = make_score("(11) 2 3", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (7, 11));
    }

    fn collect_curves(pages: &[Page]) -> Vec<(u32, u32)> {
        pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter_map(|e| match &e.content {
                GridContent::TieOrSlurCurve {
                    from_column,
                    to_column,
                } => Some((*from_column, *to_column)),
                _ => None,
            })
            .collect()
    }

    fn collect_lyric_positions(pages: &[Page]) -> Vec<(u32, String)> {
        pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter_map(|e| match &e.content {
                GridContent::Lyric { text, .. } => Some((e.position.column, text.clone())),
                _ => None,
            })
            .collect()
    }

    fn collect_underline_levels(pages: &[Page]) -> Vec<Vec<UnderlineSpan>> {
        pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter_map(|e| match &e.content {
                GridContent::DurationUnderlines { levels } => Some(levels.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn consecutive_eighth_notes_at_beat_start_share_one_underline() {
        // _2 _2 fills beat 1 (qb 0–3); 0 0 0 are quarter rests filling the rest of 4/4
        // Total: 2+2+4+4+4 = 16 quarter-beats ✓
        let score = make_score("2_ 2_ 0 0 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group");
        assert_eq!(groups[0].len(), 1, "expected one underline level");
        assert_eq!(groups[0][0].from_column, 7);
        assert_eq!(groups[0][0].to_column, 11);
    }

    #[test]
    fn eighth_rest_and_note_within_same_beat_share_one_underline() {
        // 0(4qb) _0(2qb) _2(2qb) _2(2qb) _0(2qb) 0(4qb) = 16qb ✓
        // Beat 2: _0 rest + _2 note → share one underline (same beat, rest joins beam buffer)
        // Beat 3: _2 note + _0 rest → share one underline (same beat)
        let score = make_score("0 0_ 2_ 2_ 0_ 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(
            groups.len(),
            2,
            "expected two underline groups (one per beat)"
        );
        // group[0]: beat 2 — _0 rest + _2 note
        assert_eq!(groups[0][0].from_column, 11);
        assert_eq!(groups[0][0].to_column, 15);
        // group[1]: beat 3 — _2 note + _0 rest
        assert_eq!(groups[1][0].from_column, 15);
        assert_eq!(groups[1][0].to_column, 19);
    }

    #[test]
    fn mixed_eighth_and_sixteenth_notes_produce_two_underline_levels() {
        // _1(2qb) =2(1qb) =3(1qb) fills beat 1 exactly; 0 0 0 fill 12 more qb = 16 total ✓
        // Level 1: spans all three notes (col 5–9)
        // Level 2: spans only the sixteenth sub-run =2,=3 (col 7–9)
        let score = make_score("1_ 2= 3= 0 0 0", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group");
        assert_eq!(groups[0].len(), 2, "expected two underline levels");
        assert_eq!(groups[0][0].from_column, 7);
        assert_eq!(groups[0][0].to_column, 11);
        assert_eq!(groups[0][1].from_column, 9);
        assert_eq!(groups[0][1].to_column, 11);
    }

    #[test]
    fn sixteenth_note_and_sixteenth_rests_share_one_beat_group() {
        // =1(1qb) =0(1qb) =0(1qb) =0(1qb) fills beat 1; 0 0 0 fills the remaining 12qb = 16 total ✓
        // All four fit within beat 1 → joined in one beam group with two underline levels.
        let score = make_score("1= 0= 0= 0= 0 0 0", "a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(
            groups.len(),
            1,
            "expected one beam group (note + rests share a beat)"
        );
        assert_eq!(groups[0].len(), 2, "expected two underline levels");
        // Level 1 and level 2 both span the whole beat (cols 5–9)
        assert_eq!(
            groups[0][0],
            UnderlineSpan {
                from_column: 7,
                to_column: 11,
                last_head_column: 10
            }
        );
        assert_eq!(
            groups[0][1],
            UnderlineSpan {
                from_column: 7,
                to_column: 11,
                last_head_column: 10
            }
        );
    }

    #[test]
    fn eighth_rest_underline_connects_to_following_sixteenth_notes() {
        // _0(2qb) =1(1qb) =2(1qb) fills beat 1 exactly (2+1+1=4qb); 0 0 0 fills 12 more = 16 total ✓
        // _0 rest should join the beam buffer and share the level-1 underline with =1 and =2.
        let score = make_score("0_ 1= 2= 0 0 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group spanning the beat");
        assert_eq!(groups[0].len(), 2, "expected two underline levels");
        // Level 1 spans all three (col 5–9)
        assert_eq!(groups[0][0].from_column, 7);
        assert_eq!(groups[0][0].to_column, 11);
        // Level 2 spans only =1 and =2 (col 7–9)
        assert_eq!(groups[0][1].from_column, 9);
        assert_eq!(groups[0][1].to_column, 11);
    }

    #[test]
    fn pure_sixteenth_beat_group_has_two_underlines() {
        // =1 =2 =3 =4 fills one beat exactly (4×1qb = 4qb); 0 0 0 fills 12 more qb = 16 total ✓
        // All four notes are sixteenth (underline_count=2): level-1 spans 5–9, level-2 also 5–9.
        let score = make_score("1= 2= 3= 4= 0 0 0", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group spanning the beat");
        assert_eq!(
            groups[0].len(),
            2,
            "pure-sixteenth group must produce two underline levels"
        );
        assert_eq!(
            groups[0][0],
            UnderlineSpan {
                from_column: 7,
                to_column: 11,
                last_head_column: 10
            }
        );
        assert_eq!(
            groups[0][1],
            UnderlineSpan {
                from_column: 7,
                to_column: 11,
                last_head_column: 10
            }
        );
    }

    #[test]
    fn tied_notes_share_one_lyric_syllable() {
        // 3~3 is a tie (same pitch): both notes share one syllable.
        // (33) 1 2 with lyrics "a b c":
        //   3 (col 5) → "a",  second 3 (col 9) → no lyric,  1 (col 13) → "b",  2 (col 17) → "c"
        let score = make_score("(33) 1 2", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![
                (7, "a".to_string()),
                (15, "b".to_string()),
                (19, "c".to_string())
            ],
        );
    }

    #[test]
    fn slurred_notes_each_get_a_lyric_syllable() {
        // 4~3~3: 4→3 is a slur (different pitch, each gets a syllable),
        //        3→3 is a tie (same pitch, second 3 shares the syllable of first 3).
        // So "(433) 2" with lyrics "a b c" assigns: 4→"a", first 3→"b", second 3→no lyric, 2→"c"
        let score = make_score("(433) 2", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![
                (7, "a".to_string()),
                (11, "b".to_string()),
                (19, "c".to_string())
            ],
        );
    }

    #[test]
    fn dash_lyric_is_rendered() {
        // "1 2 3 4" with lyrics "你 - 好 a": note 1→"你", note 2→"-", note 3→"好", note 4→"a"
        let score = make_score("1 2 3 4", "你 - 好 a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![
                (7, "你".to_string()),
                (11, "-".to_string()),
                (15, "好".to_string()),
                (19, "a".to_string())
            ],
        );
    }

    #[test]
    fn half_beat_note_has_duration_underline() {
        // Full 4/4 bar: 2 eighth notes separated by 3 quarter notes = 2+4+4+4+2 = 16 quarter-beats.
        // _1 and 4_ are each flushed as separate beam groups → 2 DurationUnderlines elements.
        let score = make_score("1_ 3 3 3 4_", "a b c d e");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let underlines: Vec<_> = all_elements.iter()
            .filter(|e| matches!(&e.content, GridContent::DurationUnderlines { levels } if levels.len() == 1))
            .collect();
        assert_eq!(underlines.len(), 2); // one for _1, one for 4_
    }

    #[test]
    fn dotted_half_beat_note_has_one_underline() {
        // _1* = dotted eighth (duration 3). Should get 1 underline like a plain eighth.
        // 3 + 1 + 4 + 4 + 4 = 16 quarter-beats = one full 4/4 bar.
        let score = make_score("1_. 2= 3 3 3", "a b c d e");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let underlines: Vec<_> = all_elements
            .iter()
            .filter(|e| matches!(&e.content, GridContent::DurationUnderlines { levels } if !levels.is_empty()))
            .collect();
        assert!(
            !underlines.is_empty(),
            "dotted eighth note must produce at least one underline"
        );
    }

    #[test]
    fn dotted_note_head_has_dotted_flag() {
        // _1* note head should have dotted=true in the layout element.
        let score = make_score("1_. 2= 3 3 3", "a b c d e");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let dotted_heads: Vec<_> = all_elements
            .iter()
            .filter(|e| matches!(&e.content, GridContent::NoteHead { dotted: true, .. }))
            .collect();
        assert_eq!(
            dotted_heads.len(),
            1,
            "exactly one note head should be dotted"
        );
    }

    #[test]
    fn lower_octave_note_emits_lower_octave_dots_element() {
        // "1." = pitch 1, 1-beat note (duration=4), octave -1
        // underline_count for duration=4 is 0
        let score = make_score("1, 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let lower_dots: Vec<_> = all_elements
            .iter()
            .filter(|e| matches!(e.content, GridContent::LowerOctaveDots { .. }))
            .collect();
        assert_eq!(lower_dots.len(), 1, "expected one LowerOctaveDots element");
        if let GridContent::LowerOctaveDots {
            count,
            underline_count,
        } = &lower_dots[0].content
        {
            assert_eq!(*count, 1);
            assert_eq!(*underline_count, 0, "1-beat note has 0 underlines");
        }
        assert_eq!(
            lower_dots[0].position.row, 4,
            "LowerOctaveDots must be in absolute row 4"
        );
        assert_eq!(lower_dots[0].vertical_alignment, VerticalAlignment::Top);
    }

    #[test]
    fn unchanged_labels_do_not_repeat_after_line_wrap() {
        // Wrapping is controlled by max_columns (default 28), not page width.
        // First measure: 4 (directives) + 16 (notes) + 1 (bar) = 21 cols — fits in 28.
        // Second measure: 0 + 16 + 1 = 17 cols — 21 + 17 = 38 > 28 → wraps after first measure.
        // Same time sig and BPM on second measure → no repeat labels.
        // Total TimeSignatureLabel count across the whole score should be exactly 1.
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, 300.0, A4_HEIGHT);
        let time_sig_labels: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(
            time_sig_labels.len(),
            1,
            "time signature label must not repeat on wrapped lines, got {}",
            time_sig_labels.len()
        );
    }

    #[test]
    fn part_label_and_barline_variants_exist() {
        let _ = GridContent::PartLabel {
            text: "Soprano".to_string(),
        };
        let _ = GridContent::BarLine { height_in_rows: 1 };
    }

    fn make_two_part_score(s_notes: &str, a_notes: &str) -> Score {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nSoprano = notes\nAlto = notes\n\n[score]\n(time=4/4 key=C4 bpm=120)\n{s_notes}\n{a_notes}\n"
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap()
    }

    #[test]
    fn two_part_layout_emits_part_labels() {
        let score = make_two_part_score("1 2 3 4", "5 6 7 1");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::PartLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 2, "expected one PartLabel per named part");
    }

    #[test]
    fn two_part_layout_has_note_heads_for_both_parts() {
        let score = make_two_part_score("1 2 3 4", "5 6 7 1");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let note_heads: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
            .collect();
        assert_eq!(note_heads.len(), 8, "expected 4 notes per part × 2 parts");
    }

    #[test]
    fn two_part_layout_emits_directives_on_both_parts_rows() {
        let score = make_two_part_score("1 2 3 4", "5 6 7 1");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let time_sig_labels: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(
            time_sig_labels.len(),
            2,
            "time signature label should appear on both parts' rows"
        );
    }

    #[test]
    fn single_named_part_produces_part_label() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::PartLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1);
        if let GridContent::PartLabel { text } = &labels[0].content {
            assert_eq!(text, "Melody");
        } else {
            panic!("expected PartLabel");
        }
    }

    #[test]
    fn horizontal_bar_variant_exists() {
        let _ = GridContent::HorizontalBar {
            from_column: 0,
            to_column: 12,
        };
    }

    #[test]
    fn left_bar_line_emitted_at_start_of_first_system_line() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        // label_cols=2 (named single part), header_rows=2 → row = 2+1 = 3
        let left_bars: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
            .collect();
        assert_eq!(
            left_bars.len(),
            1,
            "expected one left bar for a single system line"
        );
        assert_eq!(
            left_bars[0].position.row, 3,
            "left bar should be at row header_rows+1 = 3"
        );
    }

    #[test]
    fn left_bar_line_emitted_for_each_system_line_on_wrap() {
        // First measure: 4 (directives) + 16 (notes) + 1 (bar) = 21 cols
        // Second measure: 0 + 16 + 1 = 17 cols; 21+17=38 > 28 → wraps → 2 system lines
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, 300.0, A4_HEIGHT);
        let left_bars: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
            .collect();
        assert_eq!(left_bars.len(), 2, "expected one left bar per system line");
    }

    #[test]
    fn bottom_bar_emitted_at_end_of_system_line() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let bottom_bars: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::HorizontalBar { .. }))
            .collect();
        assert_eq!(
            bottom_bars.len(),
            1,
            "expected one bottom bar for a single system line"
        );
        // row_group_height = 4*1 = 4; row = header_rows + row_group_height = 2+4 = 6
        assert_eq!(
            bottom_bars[0].position.row, 6,
            "bottom bar row should be current_row_offset + row_group_height"
        );
        if let GridContent::HorizontalBar {
            from_column,
            to_column,
        } = &bottom_bars[0].content
        {
            assert_eq!(*from_column, 0);
            // 2 (left bar col) + 4 (directives) + 16 (notes) + 1 (end bar) + 1 (label col offset in flush?) = 24
            assert_eq!(
                *to_column, 24,
                "to_column should equal current_col at flush time"
            );
        } else {
            panic!("expected HorizontalBar");
        }
    }

    #[test]
    fn bottom_bar_emitted_for_each_system_line_on_wrap() {
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, 300.0, A4_HEIGHT);
        let bottom_bars: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::HorizontalBar { .. }))
            .collect();
        assert_eq!(
            bottom_bars.len(),
            2,
            "expected one bottom bar per system line"
        );
    }

    #[test]
    fn left_bar_line_emitted_at_correct_column_for_named_parts() {
        // Named two-part score: label_cols = ceil(label_width / row_height) = ceil(40/24) = 2
        // Left bar at column=2, height_in_rows = 1 + (2-1)*4 = 5
        let score = make_two_part_score("1 2 3 4", "5 6 7 1");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let left_bars: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
            .collect();
        assert_eq!(
            left_bars.len(),
            1,
            "expected one left bar for named two-part score"
        );
        assert_eq!(
            left_bars[0].position.row, 3,
            "left bar should be at row header_rows+1 = 3"
        );
        if let GridContent::BarLine { height_in_rows } = &left_bars[0].content {
            assert_eq!(
                *height_in_rows, 5,
                "left bar height should be row_group_height-1 = 6-1 = 5 for two-part score"
            );
        } else {
            panic!("expected BarLine");
        }
    }

    #[test]
    fn left_bar_line_has_correct_height_for_single_part() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let left_bars: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
            .collect();
        assert_eq!(left_bars.len(), 1);
        if let GridContent::BarLine { height_in_rows } = &left_bars[0].content {
            assert_eq!(
                *height_in_rows, 3,
                "single-part left bar height should be row_group_height-1 = 4-1 = 3"
            );
        } else {
            panic!("expected BarLine");
        }
    }

    #[test]
    fn bar_number_emitted_at_start_of_each_row_group() {
        // First measure: 4 (directives) + 16 (notes) + 1 (bar) = 21 cols, fits in max_columns=28.
        // Second measure: 0 + 16 + 1 = 17 cols; 21+17=38 > 28 → wraps → two row groups.
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);

        let bar_numbers: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BarNumber { .. }))
            .collect();

        // One BarNumber per row group (2 row groups total)
        assert_eq!(bar_numbers.len(), 2, "expected one BarNumber per row group");

        // First row group: bar 1, at column 2 (label_cols=2), row = header_rows = 2
        if let GridContent::BarNumber { number } = bar_numbers[0].content {
            assert_eq!(number, 1, "first row group must start at bar 1");
        }
        assert_eq!(bar_numbers[0].position.column, 2);
        assert_eq!(bar_numbers[0].position.row, 2, "row = header_rows = 2");
        assert_eq!(
            bar_numbers[0].horizontal_alignment,
            HorizontalAlignment::Left
        );
        assert_eq!(bar_numbers[0].vertical_alignment, VerticalAlignment::Bottom);

        // Second row group: bar 2, at column 2, row = header_rows + row_group_height = 2 + 4 = 6
        if let GridContent::BarNumber { number } = bar_numbers[1].content {
            assert_eq!(number, 2, "second row group must start at bar 2");
        }
        assert_eq!(bar_numbers[1].position.column, 2);
        assert_eq!(bar_numbers[1].position.row, 6, "row = 2 + 4 = 6");
    }

    #[test]
    fn bar_number_emitted_on_first_row_group_even_without_wrap() {
        // A single measure fits in one row group — no wrap occurs.
        // Bar number 1 should still be emitted at the start of that row group.
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);

        let bar_numbers: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BarNumber { .. }))
            .collect();

        assert_eq!(
            bar_numbers.len(),
            1,
            "expected one BarNumber for a single row group"
        );
        if let GridContent::BarNumber { number } = bar_numbers[0].content {
            assert_eq!(number, 1, "bar number must be 1 for the first row group");
        }
        assert_eq!(bar_numbers[0].position.column, 2);
        assert_eq!(bar_numbers[0].position.row, 2, "row = header_rows = 2");
        assert_eq!(
            bar_numbers[0].horizontal_alignment,
            HorizontalAlignment::Left
        );
        assert_eq!(bar_numbers[0].vertical_alignment, VerticalAlignment::Bottom);
    }

    #[test]
    fn cross_measure_tie_emits_right_half_arc_on_line_wrap() {
        // With default max_columns=28:
        // Measure 1: 1 (left bar col) + 4 (directives) + 16 (notes) + 1 (end bar) = 22 cols
        // Measure 2: 1 (left bar col) + 0 + 16 + 1 = 18 cols → 22+16=38 > 28 → wraps to new line
        // 3~ at col 17 in measure 1 should produce a right-half arc ending at the bar line (col 21 = 22-1).
        let score = make_score("0 0 0 (3) | 3 0 0 0", "a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert!(
            !curves.is_empty(),
            "expected right-half tie arc when cross-measure tie wraps to new line"
        );
        // The right-half arc starts at the tied note (col 19) and ends at the bar line (col 23 = 24-1).
        assert!(
            curves.iter().any(|&(from, to)| from == 19 && to == 23),
            "expected right-half arc from col 19 to col 23; got: {curves:?}"
        );
    }

    #[test]
    fn cross_measure_tie_continuation_does_not_consume_lyric_on_line_wrap() {
        // The continuation note (3 in measure 2) must NOT consume a lyric syllable
        // because prev_tie is preserved across the line boundary.
        // Only the 3~ note in measure 1 should consume a lyric.
        let score = make_score("0 0 0 (3) | 3 0 0 0", "a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let lyrics = collect_lyric_positions(&pages);
        assert_eq!(
            lyrics.len(),
            1,
            "continuation note across line break must not consume a lyric syllable; got: {lyrics:?}"
        );
        assert_eq!(lyrics[0].1, "a");
    }

    fn parse_and_layout(input: &str) -> Vec<Page> {
        let doc = parser::parse(input, "test.jianpu").unwrap();
        let score = grouper::group(doc).unwrap();
        layout(&score, A4_WIDTH, A4_HEIGHT)
    }

    #[test]
    fn section_label_element_emitted_at_correct_position() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1 2 3 4\n",
        );
        let pages = parse_and_layout(input);
        let all_elements: Vec<_> = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let label_el = all_elements.iter().find(
            |e| matches!(&e.content, GridContent::SectionLabel { text } if text == "Verse 1"),
        );
        assert!(label_el.is_some(), "expected SectionLabel element");
        let el = label_el.unwrap();
        assert_eq!(el.horizontal_alignment, HorizontalAlignment::Left);
        assert_eq!(el.vertical_alignment, VerticalAlignment::Bottom);
        let has_bar_number = all_elements
            .iter()
            .any(|e| matches!(&e.content, GridContent::BarNumber { .. }));
        assert!(
            !has_bar_number,
            "bar number must be suppressed when section label is present"
        );
    }

    #[test]
    fn chord_row_emits_chord_symbol_element() {
        use crate::layout::types::GridContent;
        let input = "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nchord = chord\nMelody = notes\n\n[score]\n(time=4/4 key=C4 bpm=120)\n1 - - -\n1---\n";
        let doc = crate::parser::parse(input, "test.jianpu").unwrap();
        let score = crate::grouper::group(doc).unwrap();
        let pages = layout(&score, 595.0, 842.0);
        let all_elements: Vec<_> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let chord_symbols: Vec<_> = all_elements
            .iter()
            .filter(|e| matches!(e.content, GridContent::ChordSymbol { .. }))
            .collect();
        assert!(
            !chord_symbols.is_empty(),
            "expected at least one ChordSymbol element"
        );
        if let GridContent::ChordSymbol { text } = &chord_symbols[0].content {
            assert_eq!(text, "1");
        }
    }

    #[test]
    fn no_section_label_when_not_declared() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
        );
        let pages = parse_and_layout(input);
        let has_label = pages[0]
            .row_groups
            .iter()
            .flat_map(|rg| rg.elements.iter())
            .any(|e| matches!(&e.content, GridContent::SectionLabel { .. }));
        assert!(!has_label, "expected no SectionLabel element");
    }
}
