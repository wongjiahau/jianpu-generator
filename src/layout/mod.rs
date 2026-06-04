use crate::ast::grouped::{NoteEvent, Score};
use crate::ast::parsed::JianPuPitch;
use crate::layout::types::*;
use crate::utils::is_cjk_char;

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
    if buffer.is_empty() {
        return;
    }
    let levels = compute_underline_levels(buffer);
    elements.push(GridElement {
        position: GridPosition {
            column: buffer[0].column,
            row: row_offset + 2,
        },
        horizontal_alignment: HorizontalAlignment::Left,
        vertical_alignment: VerticalAlignment::Top,
        content: GridContent::DurationUnderlines { levels },
    });
    buffer.clear();
}

fn compute_underline_levels(buffer: &[BeamBufferEntry]) -> Vec<UnderlineSpan> {
    let first = &buffer[0];
    let last = &buffer[buffer.len() - 1];
    // Level 1: spans all notes in the group
    let mut levels = vec![UnderlineSpan {
        from_column: first.column,
        to_column: last.column + last.duration,
    }];
    // Level 2+: one span per maximal contiguous sub-run with underline_count >= 2
    let mut run_start: Option<u32> = None;
    let mut run_end: u32 = 0;
    for entry in buffer {
        if entry.underline_count >= 2 {
            if run_start.is_none() {
                run_start = Some(entry.column);
            }
            run_end = entry.column + entry.duration;
        } else if let Some(start) = run_start.take() {
            levels.push(UnderlineSpan { from_column: start, to_column: run_end });
        }
    }
    if let Some(start) = run_start {
        levels.push(UnderlineSpan { from_column: start, to_column: run_end });
    }
    // Identical level-1 and level-2 spans are intentional: they mean "draw this span twice"
    // (e.g. a lone sixteenth note or a pure-sixteenth beat group must render two underlines).
    levels
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
const PAGE_MARGIN: f32 = 25.0;

/// A4 in points: 595 × 842.
/// Column width = row_height, row height = row_height.
pub fn layout(score: &Score, page_width_pt: f32, page_height_pt: f32) -> Vec<Page> {
    let cell = score.metadata.row_height as f32;
    let usable_width = page_width_pt - 2.0 * PAGE_MARGIN;
    let columns_per_page = (usable_width / cell) as u32;

    let num_parts = score.measures.first().map(|m| m.parts.len()).unwrap_or(1).max(1) as u32;
    let row_group_height: u32 = 4 * num_parts;

    let has_named_parts = score.measures.first()
        .map(|m| m.parts.iter().any(|p| p.name.is_some()))
        .unwrap_or(false);
    let label_cols: u32 = if has_named_parts {
        ((score.metadata.label_width as f32 / cell).ceil()) as u32
    } else {
        0
    };

    let header_rows: u32 = if score.metadata.subtitle.is_some() { 3 } else { 2 };
    let footer_rows: u32 = 1;
    let reserved_rows = header_rows + footer_rows;
    let usable_height = page_height_pt - 2.0 * PAGE_MARGIN;
    let row_groups_per_page = ((usable_height / cell) as u32 - reserved_rows) / row_group_height;

    let make_header = || Header {
        title: score.metadata.title.clone(),
        subtitle: score.metadata.subtitle.clone(),
        author: score.metadata.author.clone(),
    };

    // Collect part names for label emission (from first measure's parts)
    let part_names: Vec<Option<String>> = score.measures.first()
        .map(|m| m.parts.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default();

    let mut pages: Vec<Page> = Vec::new();
    let mut current_page_row_groups: Vec<RowGroup> = Vec::new();
    let mut current_elements: Vec<GridElement> = Vec::new();
    // current_col starts at label_cols (0 for unnamed single-part)
    let mut current_col: u32 = label_cols;
    let mut current_row_offset: u32 = header_rows;
    let mut is_line_start = true;

    // Per-part state that persists across measure boundaries
    let mut per_part_prev_tie: Vec<bool> = vec![false; num_parts as usize];
    let mut per_part_prev_pitch: Vec<Option<JianPuPitch>> = vec![None; num_parts as usize];
    let mut per_part_beam_buffer: Vec<Vec<BeamBufferEntry>> = (0..num_parts).map(|_| Vec::new()).collect();
    // pending_chain must persist across measures so cross-measure tie/slur arcs are emitted
    let mut per_part_pending_chain: Vec<Vec<(u32, JianPuPitch)>> = vec![Vec::new(); num_parts as usize];
    let mut per_part_chain_row: Vec<u32> = vec![0; num_parts as usize];

    for measure in &score.measures {
        let prefix_width = compute_prefix_width(measure);
        let measure_width = measure_column_width(measure);

        if current_col + prefix_width + measure_width > columns_per_page {
            // Flush open beam buffers for all parts
            for (part_idx, beam_buf) in per_part_beam_buffer.iter_mut().enumerate() {
                let part_row = current_row_offset + part_idx as u32 * 4;
                flush_beam_buffer(beam_buf, part_row, &mut current_elements);
            }
            // Reset tie flag on wrap; prev_pitch is not reset because it is only
            // consulted when prev_tie is true, so the stale value is never reached.
            for ppt in per_part_prev_tie.iter_mut() { *ppt = false; }
            // Drop any open chains on wrap — cross-line tie arcs are not supported.
            for chain in per_part_pending_chain.iter_mut() { chain.clear(); }

            if let Some(elements) = nonempty::NonEmpty::from_vec(std::mem::take(&mut current_elements)) {
                current_page_row_groups.push(RowGroup {
                    elements,
                    height_in_rows: row_group_height,
                    width_in_columns: current_col,
                });
            }
            current_col = label_cols;
            current_row_offset += row_group_height;
            is_line_start = true;

            if current_page_row_groups.len() >= row_groups_per_page as usize {
                if !current_page_row_groups.is_empty() {
                    pages.push(Page {
                        header: make_header(),
                        footer: Footer { page: pages.len() as u32 + 1, total: 0 },
                        row_groups: std::mem::take(&mut current_page_row_groups),
                        page_width_pt,
                    });
                }
                current_row_offset = header_rows;
            }
        }

        // Emit part labels at start of each system line
        if is_line_start && has_named_parts {
            for (part_idx, name_opt) in part_names.iter().enumerate() {
                if let Some(name) = name_opt {
                    let part_row = current_row_offset + part_idx as u32 * 4;
                    current_elements.push(GridElement {
                        position: GridPosition { column: 0, row: part_row + 1 },
                        horizontal_alignment: HorizontalAlignment::Left,
                        vertical_alignment: VerticalAlignment::Center,
                        content: GridContent::PartLabel { text: name.clone() },
                    });
                }
            }
        }
        is_line_start = false;

        // Emit directives for every part at their respective row offsets
        let directive_col_start = current_col;
        let mut directive_advance = 0u32;

        for part_idx in 0..(num_parts as usize) {
            let part_row = current_row_offset + part_idx as u32 * 4;
            let mut dc = directive_col_start;

            if let Some(ts) = &measure.time_signature {
                current_elements.push(GridElement {
                    position: GridPosition { column: dc, row: part_row + 1 },
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                    content: GridContent::TimeSignatureLabel {
                        numerator: ts.numerator,
                        denominator: ts.denominator,
                    },
                });
                dc += 2;
                if part_idx == 0 { directive_advance += 2; }
            }

            if let Some(bpm) = measure.bpm {
                current_elements.push(GridElement {
                    position: GridPosition { column: dc, row: part_row + 1 },
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                    content: GridContent::BpmLabel { bpm },
                });
                if part_idx == 0 { directive_advance += 2; }
            }
        }

        current_col = directive_col_start + directive_advance;
        let note_col_start = current_col;

        // Compute max notes width for bar line placement
        let max_notes_width: u32 = measure.parts.iter().map(|part| {
            part.notes.events.iter().map(|n| match n {
                NoteEvent::Note(note) => note.duration,
                NoteEvent::Rest(rest) => rest.duration,
            }).sum::<u32>()
        }).max().unwrap_or(0);

        // Emit notes/lyrics for each part
        for (part_idx, part_slice) in measure.parts.iter().enumerate() {
            let part_row = current_row_offset + part_idx as u32 * 4;
            let mut col = note_col_start;
            let measure_col_start_for_part = note_col_start;

            let pending_chain = &mut per_part_pending_chain[part_idx];
            let chain_row_ref = &mut per_part_chain_row[part_idx];
            if pending_chain.is_empty() { *chain_row_ref = part_row + 1; }
            let beam_buf = &mut per_part_beam_buffer[part_idx];
            let prev_tie = &mut per_part_prev_tie[part_idx];
            let prev_pitch = &mut per_part_prev_pitch[part_idx];

            let mut lyrics_iter = part_slice.lyrics.as_ref().map(|l| l.syllables.iter());

            for note_event in &part_slice.notes.events {
                match note_event {
                    NoteEvent::Note(note) => {
                        // Note head (row +1)
                        current_elements.push(GridElement {
                            position: GridPosition { column: col, row: part_row + 1 },
                            horizontal_alignment: HorizontalAlignment::Center,
                            vertical_alignment: VerticalAlignment::Center,
                            content: GridContent::NoteHead { pitch: note.pitch.clone(), octave: note.octave },
                        });

                        // Lower octave dots (row +2)
                        if note.octave < 0 {
                            current_elements.push(GridElement {
                                position: GridPosition { column: col, row: part_row + 2 },
                                horizontal_alignment: HorizontalAlignment::Center,
                                vertical_alignment: VerticalAlignment::Bottom,
                                content: GridContent::LowerOctaveDots { count: (-note.octave) as u32 },
                            });
                        }

                        // Extension dashes (row +1)
                        if note.duration > 4 {
                            let extra_beats = (note.duration - 4) / 4;
                            for i in 0..extra_beats {
                                current_elements.push(GridElement {
                                    position: GridPosition { column: col + 4 + i * 4, row: part_row + 1 },
                                    horizontal_alignment: HorizontalAlignment::Center,
                                    vertical_alignment: VerticalAlignment::Center,
                                    content: GridContent::Extension,
                                });
                            }
                        }

                        let underline_count = match note.duration {
                            1 => 2,
                            2 => 1,
                            _ => 0,
                        };

                        if underline_count == 0 {
                            flush_beam_buffer(beam_buf, part_row, &mut current_elements);
                        }

                        pending_chain.push((col, note.pitch.clone()));

                        // Lyric (row +3)
                        let is_tie_continuation = *prev_tie && prev_pitch.as_ref() == Some(&note.pitch);
                        if !is_tie_continuation {
                            if let Some(ref mut iter) = lyrics_iter {
                                if let Some(syllable) = iter.next() {
                                    let is_cjk = syllable.text.chars().next().map(|c| is_cjk_char(c)).unwrap_or(false);
                                    current_elements.push(GridElement {
                                        position: GridPosition { column: col, row: part_row + 3 },
                                        horizontal_alignment: HorizontalAlignment::Center,
                                        vertical_alignment: VerticalAlignment::Top,
                                        content: GridContent::Lyric { text: syllable.text.clone(), is_cjk },
                                    });
                                }
                            }
                        }
                        *prev_tie = note.tie;
                        *prev_pitch = Some(note.pitch.clone());

                        if underline_count > 0 {
                            beam_buf.push(BeamBufferEntry {
                                column: col,
                                underline_count,
                                duration: note.duration,
                            });
                        }

                        col += note.duration;

                        let beat_position = col - measure_col_start_for_part;
                        if underline_count > 0 && beat_position % 4 == 0 {
                            flush_beam_buffer(beam_buf, part_row, &mut current_elements);
                        }

                        if !note.tie {
                            flush_chain(pending_chain, *chain_row_ref, &mut current_elements);
                            pending_chain.clear();
                        }
                    }
                    NoteEvent::Rest(rest) => {
                        flush_beam_buffer(beam_buf, part_row, &mut current_elements);
                        current_elements.push(GridElement {
                            position: GridPosition { column: col, row: part_row + 1 },
                            horizontal_alignment: HorizontalAlignment::Center,
                            vertical_alignment: VerticalAlignment::Center,
                            content: GridContent::Rest,
                        });
                        col += rest.duration;
                        *prev_tie = false;
                    }
                }
            }

            flush_beam_buffer(beam_buf, part_row, &mut current_elements);
        }

        // Bar line spanning all parts
        let bar_col = note_col_start + max_notes_width;
        let bar_height = 1 + (num_parts - 1) * 4;
        current_elements.push(GridElement {
            position: GridPosition { column: bar_col, row: current_row_offset + 1 },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Center,
            content: GridContent::BarLine { height_in_rows: bar_height },
        });
        current_col = bar_col + 1;
    }

    // Flush remaining elements
    if let Some(elements) = nonempty::NonEmpty::from_vec(std::mem::take(&mut current_elements)) {
        current_page_row_groups.push(RowGroup {
            elements,
            height_in_rows: row_group_height,
            width_in_columns: current_col,
        });
    }
    if !current_page_row_groups.is_empty() {
        pages.push(Page {
            header: make_header(),
            footer: Footer { page: pages.len() as u32 + 1, total: 0 },
            row_groups: std::mem::take(&mut current_page_row_groups),
            page_width_pt,
        });
    }

    if pages.is_empty() {
        pages.push(Page {
            header: make_header(),
            footer: Footer { page: 1, total: 1 },
            row_groups: Vec::new(),
            page_width_pt,
        });
    }

    let total = pages.len() as u32;
    for page in &mut pages {
        page.footer.total = total;
    }

    pages
}

/// Emit tie/slur arcs for a completed chain of `~`-connected notes.
///
/// Rules:
/// - If the chain contains any pitch change → one **slur** arc from first to last note.
/// - For every consecutive same-pitch pair within the chain → one **tie** arc between them.
fn flush_chain(chain: &[(u32, JianPuPitch)], chain_row: u32, elements: &mut Vec<GridElement>) {
    if chain.len() <= 1 {
        return;
    }

    let has_pitch_change = chain.windows(2).any(|w| w[0].1 != w[1].1);

    if has_pitch_change {
        // One slur spanning the entire chain
        elements.push(GridElement {
            position: GridPosition { column: chain[0].0, row: chain_row },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            content: GridContent::TieOrSlurCurve {
                from_column: chain[0].0,
                to_column: chain.last().unwrap().0,
            },
        });
    }

    // Tie arc for each consecutive same-pitch pair
    for w in chain.windows(2) {
        if w[0].1 == w[1].1 {
            elements.push(GridElement {
                position: GridPosition { column: w[0].0, row: chain_row },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Top,
                content: GridContent::TieOrSlurCurve {
                    from_column: w[0].0,
                    to_column: w[1].0,
                },
            });
        }
    }
}

fn measure_column_width(measure: &crate::ast::grouped::MultiPartMeasure) -> u32 {
    let max_notes: u32 = measure.parts.iter().map(|part| {
        part.notes.events.iter().map(|n| match n {
            NoteEvent::Note(note) => note.duration,
            NoteEvent::Rest(rest) => rest.duration,
        }).sum::<u32>()
    }).max().unwrap_or(0);
    max_notes + 1 // +1 for bar line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::grouper;

    fn make_score(score_str: &str, lyrics_str: &str) -> Score {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n4/4 {}\n\n[lyrics]\n{}\n",
            score_str, lyrics_str
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap()
    }

    fn make_score_raw(score_section: &str, lyrics_str: &str) -> Score {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[score]\n{}\n\n[lyrics]\n{}\n",
            score_section, lyrics_str
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap()
    }

    #[test]
    fn first_measure_emits_time_signature_label_at_column_zero() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].position.column, 0);
        if let GridContent::TimeSignatureLabel { numerator, denominator } = &labels[0].content {
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
        let labels: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].position.column, 2);
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
        let note_heads: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
            .collect();
        assert_eq!(note_heads[0].position.column, 4);
    }

    #[test]
    fn unchanged_time_signature_emits_no_second_label() {
        let score = make_score("1 2 3 4 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1, "only one time signature label expected for two measures with identical time signature on the same line");
    }

    #[test]
    fn unchanged_bpm_emits_no_second_label() {
        let score = make_score("1 2 3 4 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 1, "only one BPM label expected for two measures with identical BPM on the same line");
    }

    #[test]
    fn time_signature_change_emits_second_label() {
        let score = make_score_raw("4/4 1 2 3 4 3/4 1 2 3", "a b c d e f g");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 2, "expected one label per distinct time signature");
    }

    #[test]
    fn bpm_change_emits_second_label() {
        let score = make_score_raw("4/4 bpm=120 1 2 3 4 bpm=90 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 2, "expected one BPM label per distinct BPM value");
    }

    const A4_WIDTH: f32 = 595.0;  // points
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
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let note_heads: Vec<_> = all_elements.iter()
            .filter(|e| matches!(e.content, GridContent::NoteHead { .. }))
            .collect();
        assert_eq!(note_heads.len(), 4);
    }

    #[test]
    fn lyrics_are_present() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let lyrics: Vec<_> = all_elements.iter()
            .filter(|e| matches!(e.content, GridContent::Lyric { .. }))
            .collect();
        assert_eq!(lyrics.len(), 4);
    }

    #[test]
    fn two_different_notes_emit_one_slur() {
        // 1~ 2: different pitches → one slur from col 0 to col 4
        let score = make_score("1~ 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (4, 8));
    }

    #[test]
    fn three_note_slur_emits_one_curve() {
        // 3~2~1: all different pitches → one slur from col 0 to col 8
        let score = make_score("3~2~1 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (4, 12));
    }

    #[test]
    fn mixed_chain_emits_slur_and_tie() {
        // 4~3~3 2: chain [4@0, 3@4, 3@8]
        // → one slur from 0 to 8 (pitch change exists)
        // → one tie from 4 to 8 (same-pitch pair 3~3)
        let score = make_score("4~3~3 2", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let mut curves = collect_curves(&pages);
        curves.sort();
        assert_eq!(curves.len(), 2);
        assert_eq!(curves[0], (4, 12)); // slur
        assert_eq!(curves[1], (8, 12)); // tie
    }

    #[test]
    fn same_pitch_chain_emits_only_tie() {
        // 1~1 2 3: same pitches → one tie, no slur
        let score = make_score("1~1 2 3", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0], (4, 8));
    }

    fn collect_curves(pages: &[Page]) -> Vec<(u32, u32)> {
        pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter_map(|e| match &e.content {
                GridContent::TieOrSlurCurve { from_column, to_column } => Some((*from_column, *to_column)),
                _ => None,
            })
            .collect()
    }

    fn collect_lyric_positions(pages: &[Page]) -> Vec<(u32, String)> {
        pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter_map(|e| match &e.content {
                GridContent::Lyric { text, .. } => Some((e.position.column, text.clone())),
                _ => None,
            })
            .collect()
    }

    fn collect_underline_levels(pages: &[Page]) -> Vec<Vec<UnderlineSpan>> {
        pages.iter()
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
        let score = make_score("_2 _2 0 0 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group");
        assert_eq!(groups[0].len(), 1, "expected one underline level");
        assert_eq!(groups[0][0].from_column, 4);
        assert_eq!(groups[0][0].to_column, 8);
    }

    #[test]
    fn eighth_notes_straddling_beat_boundary_produce_separate_underlines() {
        // 0(4qb) _0(2qb) _2(2qb) _2(2qb) _0(2qb) 0(4qb) = 16qb ✓
        // First _2 starts at qb 6 (mid-beat-2), ends at qb 8 (beat boundary) → flushed alone
        // Second _2 starts at qb 8, ends at qb 10 → flushed alone when _0 rest arrives
        let score = make_score("0 _0 _2 _2 _0 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 2, "expected two separate underline groups");
        assert_eq!(groups[0][0].from_column, 10);
        assert_eq!(groups[0][0].to_column, 12);
        assert_eq!(groups[1][0].from_column, 12);
        assert_eq!(groups[1][0].to_column, 14);
    }

    #[test]
    fn mixed_eighth_and_sixteenth_notes_produce_two_underline_levels() {
        // _1(2qb) =2(1qb) =3(1qb) fills beat 1 exactly; 0 0 0 fill 12 more qb = 16 total ✓
        // Level 1: spans all three notes (col 0–4)
        // Level 2: spans only the sixteenth sub-run =2,=3 (col 2–4)
        let score = make_score("_1 =2 =3 0 0 0", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group");
        assert_eq!(groups[0].len(), 2, "expected two underline levels");
        assert_eq!(groups[0][0].from_column, 4);
        assert_eq!(groups[0][0].to_column, 8);
        assert_eq!(groups[0][1].from_column, 6);
        assert_eq!(groups[0][1].to_column, 8);
    }

    #[test]
    fn lone_sixteenth_note_has_two_underlines() {
        // =1(1qb) and fill with quarter rests: need 15 more qb but rests are 4qb each = can't hit 16.
        // Use: =1 =0 =0 =0 0 0 0 = 1+1+1+1+4+4+4 = 16qb ✓
        // Only =1 is a note (pitch); =0 are sixteenth rests → flush before each rest.
        // So =1 is a lone sixteenth in the buffer → produces level-1 and level-2 spans both {0,1}.
        let score = make_score("=1 =0 =0 =0 0 0 0", "a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group for the lone sixteenth");
        assert_eq!(groups[0].len(), 2, "lone sixteenth must produce two underline levels");
        assert_eq!(groups[0][0], UnderlineSpan { from_column: 4, to_column: 5 });
        assert_eq!(groups[0][1], UnderlineSpan { from_column: 4, to_column: 5 });
    }

    #[test]
    fn pure_sixteenth_beat_group_has_two_underlines() {
        // =1 =2 =3 =4 fills one beat exactly (4×1qb = 4qb); 0 0 0 fills 12 more qb = 16 total ✓
        // All four notes are sixteenth (underline_count=2): level-1 spans 0–4, level-2 also 0–4.
        let score = make_score("=1 =2 =3 =4 0 0 0", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 1, "expected one beam group spanning the beat");
        assert_eq!(groups[0].len(), 2, "pure-sixteenth group must produce two underline levels");
        assert_eq!(groups[0][0], UnderlineSpan { from_column: 4, to_column: 8 });
        assert_eq!(groups[0][1], UnderlineSpan { from_column: 4, to_column: 8 });
    }

    #[test]
    fn tied_notes_share_one_lyric_syllable() {
        // 3~3 is a tie (same pitch): both notes share one syllable.
        // 3~3 1 2 with lyrics "a b c":
        //   3 (col 0) → "a",  second 3 (col 4) → no lyric,  1 (col 8) → "b",  2 (col 12) → "c"
        let score = make_score("3~3 1 2", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![(4, "a".to_string()), (12, "b".to_string()), (16, "c".to_string())],
        );
    }

    #[test]
    fn slurred_notes_each_get_a_lyric_syllable() {
        // 4~3~3: 4→3 is a slur (different pitch, each gets a syllable),
        //        3→3 is a tie (same pitch, second 3 shares the syllable of first 3).
        // So "4~3~3 2" with lyrics "a b c" assigns: 4→"a", first 3→"b", second 3→no lyric, 2→"c"
        let score = make_score("4~3~3 2", "a b c");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![(4, "a".to_string()), (8, "b".to_string()), (16, "c".to_string())],
        );
    }

    #[test]
    fn dash_lyric_is_rendered() {
        // "1 2 3 4" with lyrics "你 - 好 a": note 1→"你", note 2→"-", note 3→"好", note 4→"a"
        let score = make_score("1 2 3 4", "你 - 好 a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        assert_eq!(
            collect_lyric_positions(&pages),
            vec![(4, "你".to_string()), (8, "-".to_string()), (12, "好".to_string()), (16, "a".to_string())],
        );
    }

    #[test]
    fn half_beat_note_has_duration_underline() {
        let score = make_score("_1 2 3 _4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let underlines: Vec<_> = all_elements.iter()
            .filter(|e| matches!(&e.content, GridContent::DurationUnderlines { levels } if levels.len() == 1))
            .collect();
        assert_eq!(underlines.len(), 2); // _1 and _4
    }

    #[test]
    fn lower_octave_note_emits_lower_octave_dots_element() {
        // "1." means pitch 1 with one trailing dot = octave -1
        let score = make_score("1. 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let lower_dots: Vec<_> = all_elements.iter()
            .filter(|e| matches!(e.content, GridContent::LowerOctaveDots { .. }))
            .collect();
        assert_eq!(lower_dots.len(), 1, "expected one LowerOctaveDots element");
        if let GridContent::LowerOctaveDots { count } = &lower_dots[0].content {
            assert_eq!(*count, 1);
        }
        // First row-group starts at row offset = header_rows (2), so underline sub-row (+2) → absolute row 4
        assert_eq!(lower_dots[0].position.row, 4, "LowerOctaveDots must be in absolute row 4 (header_rows + underline sub-row)");
        assert_eq!(lower_dots[0].vertical_alignment, VerticalAlignment::Bottom);
    }

    #[test]
    fn unchanged_labels_do_not_repeat_after_line_wrap() {
        // Use a narrow page so measures wrap across multiple row groups.
        // With cell_size=24 and page_width=300: columns_per_page = 12.
        // First measure: 2+2+16+1 = 21 > 12 → wraps before placing notes.
        // After wrap the first measure is placed (still same time sig, same BPM).
        // Second measure: same time sig, same BPM → no prefix labels.
        // Total TimeSignatureLabel count across the whole score should be exactly 1.
        let score = make_score("1 2 3 4 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, 300.0, A4_HEIGHT);
        let time_sig_labels: Vec<_> = pages.iter()
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
        let _ = GridContent::PartLabel { text: "Soprano".to_string() };
        let _ = GridContent::BarLine { height_in_rows: 1 };
    }

    fn make_two_part_score(s_notes: &str, a_notes: &str) -> Score {
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n[score:Soprano]\n4/4 {}\n[score:Alto]\n{}\n",
            s_notes, a_notes
        );
        let doc = parser::parse(&input, "test.jianpu").unwrap();
        grouper::group(doc).unwrap()
    }

    #[test]
    fn two_part_layout_emits_part_labels() {
        let score = make_two_part_score("1 2 3 4", "5 6 7 1");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
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
        let note_heads: Vec<_> = pages.iter()
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
        let time_sig_labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
            .collect();
        assert_eq!(time_sig_labels.len(), 2, "time signature label should appear on both parts' rows");
    }

    #[test]
    fn single_unnamed_part_produces_no_part_labels() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let labels: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::PartLabel { .. }))
            .collect();
        assert_eq!(labels.len(), 0);
    }
}
