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
/// Row height in points = score.metadata.row_height. Column width varies per row (justified).
pub fn layout(score: &Score, page_width_pt: f32, page_height_pt: f32) -> Vec<Page> {
    let row_height = score.metadata.row_height as f32;
    let columns_per_row = score.metadata.max_columns;

    let num_parts = score.measures.first().map(|m| m.parts.len()).unwrap_or(1).max(1) as u32;
    let row_group_height: u32 = 4 * num_parts;
    let bar_height: u32 = row_group_height - 1;

    let has_named_parts = score.measures.first()
        .map(|m| m.parts.iter().any(|p| p.name.is_some()))
        .unwrap_or(false);
    let label_cols: u32 = if has_named_parts {
        ((score.metadata.label_width as f32 / row_height).ceil()) as u32
    } else {
        0
    };

    let header_rows: u32 = if score.metadata.subtitle.is_some() { 3 } else { 2 };
    let footer_rows: u32 = 1;
    let reserved_rows = header_rows + footer_rows;
    let usable_height = page_height_pt - 2.0 * PAGE_MARGIN;
    let row_groups_per_page = ((usable_height / row_height) as u32 - reserved_rows) / row_group_height;

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
    let mut bar_number: u32 = 1;

    // Per-part state that persists across measure boundaries
    let mut per_part_prev_tie: Vec<bool> = vec![false; num_parts as usize];
    let mut per_part_prev_pitch: Vec<Option<JianPuPitch>> = vec![None; num_parts as usize];
    let mut per_part_beam_buffer: Vec<Vec<BeamBufferEntry>> = (0..num_parts).map(|_| Vec::new()).collect();
    // pending_chain must persist across measures so cross-measure tie/slur arcs are emitted
    let mut per_part_pending_chain: Vec<Vec<(u32, JianPuPitch)>> = vec![Vec::new(); num_parts as usize];
    let mut per_part_chain_row: Vec<u32> = vec![0; num_parts as usize];
    // Tracks a tie pitch that crossed a line boundary, so a left-half arc can be drawn on the new line.
    let mut per_part_cross_line_tie: Vec<Option<JianPuPitch>> = vec![None; num_parts as usize];

    for measure in &score.measures {
        let prefix_width = compute_prefix_width(measure);
        let measure_width = measure_column_width(measure);

        if current_col + prefix_width + measure_width > columns_per_row {
            // Flush open beam buffers for all parts
            for (part_idx, beam_buf) in per_part_beam_buffer.iter_mut().enumerate() {
                let part_row = current_row_offset + part_idx as u32 * 4;
                flush_beam_buffer(beam_buf, part_row, &mut current_elements);
            }
            // Emit right-half tie arcs for open chains crossing the line boundary.
            // prev_tie is intentionally NOT reset here — it persists to the next line
            // so that the continuation note skips consuming a lyric syllable.
            for (part_idx, chain) in per_part_pending_chain.iter().enumerate() {
                if !chain.is_empty() {
                    let chain_row = per_part_chain_row[part_idx];
                    let last = chain.last().unwrap();
                    let to_col = current_col.saturating_sub(1);
                    if last.0 < to_col {
                        current_elements.push(GridElement {
                            position: GridPosition { column: last.0, row: chain_row },
                            horizontal_alignment: HorizontalAlignment::Left,
                            vertical_alignment: VerticalAlignment::Top,
                            content: GridContent::TieOrSlurCurve {
                                from_column: last.0,
                                to_column: to_col,
                            },
                        });
                    }
                    per_part_cross_line_tie[part_idx] = Some(last.1.clone());
                }
            }
            for chain in per_part_pending_chain.iter_mut() { chain.clear(); }

            // Bottom system bar for the row group being flushed
            current_elements.push(GridElement {
                position: GridPosition { column: 0, row: current_row_offset + row_group_height },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Top,
                content: GridContent::HorizontalBar { from_column: 0, to_column: current_col },
            });

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

        // Left system bar at start of each system line
        if is_line_start {
            current_elements.push(GridElement {
                position: GridPosition { column: label_cols, row: current_row_offset + 1 },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Center,
                content: GridContent::BarLine { height_in_rows: bar_height },
            });
            current_elements.push(GridElement {
                position: GridPosition { column: label_cols, row: current_row_offset },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Bottom,
                content: GridContent::BarNumber { number: bar_number },
            });
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
            let cross_line_tie = &mut per_part_cross_line_tie[part_idx];

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

                        // Left-half arc for a tie that crossed a line boundary
                        if cross_line_tie.is_some() {
                            if is_tie_continuation && col > label_cols {
                                current_elements.push(GridElement {
                                    position: GridPosition { column: label_cols, row: *chain_row_ref },
                                    horizontal_alignment: HorizontalAlignment::Left,
                                    vertical_alignment: VerticalAlignment::Top,
                                    content: GridContent::TieOrSlurCurve {
                                        from_column: label_cols,
                                        to_column: col,
                                    },
                                });
                            }
                            *cross_line_tie = None;
                        }

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
                        let rest_underline_count = match rest.duration {
                            1 => 2,
                            2 => 1,
                            _ => 0,
                        };
                        if rest_underline_count > 0 {
                            let span = UnderlineSpan { from_column: col, to_column: col + rest.duration };
                            let mut levels = vec![span.clone()];
                            if rest_underline_count >= 2 {
                                levels.push(span);
                            }
                            current_elements.push(GridElement {
                                position: GridPosition { column: col, row: part_row + 2 },
                                horizontal_alignment: HorizontalAlignment::Left,
                                vertical_alignment: VerticalAlignment::Top,
                                content: GridContent::DurationUnderlines { levels },
                            });
                        }
                        col += rest.duration;
                        *prev_tie = false;
                        *cross_line_tie = None;
                    }
                }
            }

            flush_beam_buffer(beam_buf, part_row, &mut current_elements);
        }

        // Bar line spanning all parts
        let bar_col = note_col_start + max_notes_width;
        current_elements.push(GridElement {
            position: GridPosition { column: bar_col, row: current_row_offset + 1 },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Center,
            content: GridContent::BarLine { height_in_rows: bar_height },
        });
        current_col = bar_col + 1;
        bar_number += 1;
    }

    // Bottom system bar for the last row group
    if !current_elements.is_empty() {
        current_elements.push(GridElement {
            position: GridPosition { column: 0, row: current_row_offset + row_group_height },
            horizontal_alignment: HorizontalAlignment::Left,
            vertical_alignment: VerticalAlignment::Top,
            content: GridContent::HorizontalBar { from_column: 0, to_column: current_col },
        });
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

    /// Build a single-part score with lyrics from bar-separated notes (use `|` to separate bars).
    /// All lyrics syllables are placed in the first bar's lyrics row; the grouper distributes them
    /// across measures. Subsequent bars omit the lyrics line (parser pads with empty).
    fn make_score(score_str: &str, lyrics_str: &str) -> Score {
        let bars: Vec<&str> = score_str.split('|').map(str::trim).filter(|s| !s.is_empty()).collect();
        let mut score_content = String::new();
        score_content.push_str("(time=4/4 key=C4 bpm=120)\n");
        for (i, bar) in bars.iter().enumerate() {
            score_content.push_str(bar);
            score_content.push('\n');
            if i == 0 {
                // Place all lyrics in first bar; grouper distributes across measures
                score_content.push_str(lyrics_str);
                score_content.push('\n');
            }
            // Subsequent bars omit the lyrics line; interleaved parser pads with empty string
            score_content.push('\n'); // blank line separating bar groups
        }
        let input = format!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n[score]\n{}",
            score_content
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
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n[score]\n{}",
            score_section
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
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
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
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
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
        // Two bars: first 4/4 (4 quarter notes), second 3/4 (3 quarter notes), each with lyrics.
        let score = make_score_raw(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(time=3/4)\n1 2 3\ne f g\n",
            "",
        );
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
        // Two bars: first at bpm=120, second at bpm=90, each with lyrics.
        let score = make_score_raw(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n\n(bpm=90)\n5 6 7 1\ne f g h\n",
            "",
        );
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
        // The two _0 rests also produce their own underlines (groups[0] and groups[3]).
        let score = make_score("0 _0 _2 _2 _0 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 4, "expected four underline groups (2 rests + 2 notes)");
        // groups[0]: first _0 rest underline
        assert_eq!(groups[0][0].from_column, 8);
        assert_eq!(groups[0][0].to_column, 10);
        // groups[1] and groups[2]: note underlines
        assert_eq!(groups[1][0].from_column, 10);
        assert_eq!(groups[1][0].to_column, 12);
        assert_eq!(groups[2][0].from_column, 12);
        assert_eq!(groups[2][0].to_column, 14);
        // groups[3]: second _0 rest underline
        assert_eq!(groups[3][0].from_column, 14);
        assert_eq!(groups[3][0].to_column, 16);
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
        // Each =0 rest also produces its own two-level underline.
        let score = make_score("=1 =0 =0 =0 0 0 0", "a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let groups = collect_underline_levels(&pages);
        assert_eq!(groups.len(), 4, "expected four underline groups (1 note + 3 sixteenth rests)");
        // groups[0]: lone =1 note — two levels at same span
        assert_eq!(groups[0].len(), 2, "lone sixteenth must produce two underline levels");
        assert_eq!(groups[0][0], UnderlineSpan { from_column: 4, to_column: 5 });
        assert_eq!(groups[0][1], UnderlineSpan { from_column: 4, to_column: 5 });
        // groups[1..3]: =0 rests — each two levels at their own span
        assert_eq!(groups[1][0], UnderlineSpan { from_column: 5, to_column: 6 });
        assert_eq!(groups[1][1], UnderlineSpan { from_column: 5, to_column: 6 });
        assert_eq!(groups[2][0], UnderlineSpan { from_column: 6, to_column: 7 });
        assert_eq!(groups[3][0], UnderlineSpan { from_column: 7, to_column: 8 });
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
        // Full 4/4 bar: 2 eighth notes separated by 3 quarter notes = 2+4+4+4+2 = 16 quarter-beats.
        // _1 and _4 are each flushed as separate beam groups → 2 DurationUnderlines elements.
        let score = make_score("_1 3 3 3 _4", "a b c d e");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let all_elements: Vec<_> = pages[0].row_groups.iter()
            .flat_map(|rg| rg.elements.iter())
            .collect();
        let underlines: Vec<_> = all_elements.iter()
            .filter(|e| matches!(&e.content, GridContent::DurationUnderlines { levels } if levels.len() == 1))
            .collect();
        assert_eq!(underlines.len(), 2); // one for _1, one for _4
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
        // Wrapping is controlled by max_columns (default 28), not page width.
        // First measure: 4 (directives) + 16 (notes) + 1 (bar) = 21 cols — fits in 28.
        // Second measure: 0 + 16 + 1 = 17 cols — 21 + 17 = 38 > 28 → wraps after first measure.
        // Same time sig and BPM on second measure → no repeat labels.
        // Total TimeSignatureLabel count across the whole score should be exactly 1.
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
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
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano notes:Alto\n\n[score]\n(time=4/4 key=C4 bpm=120)\n{}\n{}\n",
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

    #[test]
    fn horizontal_bar_variant_exists() {
        let _ = GridContent::HorizontalBar { from_column: 0, to_column: 10 };
    }

    #[test]
    fn left_bar_line_emitted_at_start_of_first_system_line() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        // label_cols=0 (unnamed single part), header_rows=2 → row = 2+1 = 3
        let left_bars: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 0)
            .collect();
        assert_eq!(left_bars.len(), 1, "expected one left bar for a single system line");
        assert_eq!(left_bars[0].position.row, 3, "left bar should be at row header_rows+1 = 3");
    }

    #[test]
    fn left_bar_line_emitted_for_each_system_line_on_wrap() {
        // First measure: 4 (directives) + 16 (notes) + 1 (bar) = 21 cols
        // Second measure: 0 + 16 + 1 = 17 cols; 21+17=38 > 28 → wraps → 2 system lines
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, 300.0, A4_HEIGHT);
        let left_bars: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 0)
            .collect();
        assert_eq!(left_bars.len(), 2, "expected one left bar per system line");
    }

    #[test]
    fn bottom_bar_emitted_at_end_of_system_line() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let bottom_bars: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::HorizontalBar { .. }))
            .collect();
        assert_eq!(bottom_bars.len(), 1, "expected one bottom bar for a single system line");
        // row_group_height = 4*1 = 4; row = header_rows + row_group_height = 2+4 = 6
        assert_eq!(bottom_bars[0].position.row, 6, "bottom bar row should be current_row_offset + row_group_height");
        if let GridContent::HorizontalBar { from_column, to_column } = &bottom_bars[0].content {
            assert_eq!(*from_column, 0);
            // 4 (directives) + 16 (notes) + 1 (bar) = 21
            assert_eq!(*to_column, 21, "to_column should equal current_col at flush time");
        } else {
            panic!("expected HorizontalBar");
        }
    }

    #[test]
    fn bottom_bar_emitted_for_each_system_line_on_wrap() {
        let score = make_score("1 2 3 4 | 5 6 7 1", "a b c d e f g h");
        let pages = layout(&score, 300.0, A4_HEIGHT);
        let bottom_bars: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::HorizontalBar { .. }))
            .collect();
        assert_eq!(bottom_bars.len(), 2, "expected one bottom bar per system line");
    }

    #[test]
    fn left_bar_line_emitted_at_correct_column_for_named_parts() {
        // Named two-part score: label_cols = ceil(label_width / row_height) = ceil(40/24) = 2
        // Left bar at column=2, height_in_rows = 1 + (2-1)*4 = 5
        let score = make_two_part_score("1 2 3 4", "5 6 7 1");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let left_bars: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 2)
            .collect();
        assert_eq!(left_bars.len(), 1, "expected one left bar for named two-part score");
        assert_eq!(left_bars[0].position.row, 3, "left bar should be at row header_rows+1 = 3");
        if let GridContent::BarLine { height_in_rows } = &left_bars[0].content {
            assert_eq!(*height_in_rows, 7, "left bar height should be row_group_height-1 = 8-1 = 7 for two-part score");
        } else {
            panic!("expected BarLine");
        }
    }

    #[test]
    fn left_bar_line_has_correct_height_for_single_part() {
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let left_bars: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 0)
            .collect();
        assert_eq!(left_bars.len(), 1);
        if let GridContent::BarLine { height_in_rows } = &left_bars[0].content {
            assert_eq!(*height_in_rows, 3, "single-part left bar height should be row_group_height-1 = 4-1 = 3");
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

        let bar_numbers: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BarNumber { .. }))
            .collect();

        // One BarNumber per row group (2 row groups total)
        assert_eq!(bar_numbers.len(), 2, "expected one BarNumber per row group");

        // First row group: bar 1, at column 0 (label_cols=0), row = header_rows = 2
        if let GridContent::BarNumber { number } = bar_numbers[0].content {
            assert_eq!(number, 1, "first row group must start at bar 1");
        }
        assert_eq!(bar_numbers[0].position.column, 0);
        assert_eq!(bar_numbers[0].position.row, 2, "row = header_rows = 2");
        assert_eq!(bar_numbers[0].horizontal_alignment, HorizontalAlignment::Left);
        assert_eq!(bar_numbers[0].vertical_alignment, VerticalAlignment::Bottom);

        // Second row group: bar 2, at column 0, row = header_rows + row_group_height = 2 + 4 = 6
        if let GridContent::BarNumber { number } = bar_numbers[1].content {
            assert_eq!(number, 2, "second row group must start at bar 2");
        }
        assert_eq!(bar_numbers[1].position.column, 0);
        assert_eq!(bar_numbers[1].position.row, 6, "row = 2 + 4 = 6");
    }

    #[test]
    fn bar_number_emitted_on_first_row_group_even_without_wrap() {
        // A single measure fits in one row group — no wrap occurs.
        // Bar number 1 should still be emitted at the start of that row group.
        let score = make_score("1 2 3 4", "a b c d");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);

        let bar_numbers: Vec<_> = pages.iter()
            .flat_map(|p| p.row_groups.iter())
            .flat_map(|rg| rg.elements.iter())
            .filter(|e| matches!(e.content, GridContent::BarNumber { .. }))
            .collect();

        assert_eq!(bar_numbers.len(), 1, "expected one BarNumber for a single row group");
        if let GridContent::BarNumber { number } = bar_numbers[0].content {
            assert_eq!(number, 1, "bar number must be 1 for the first row group");
        }
        assert_eq!(bar_numbers[0].position.column, 0);
        assert_eq!(bar_numbers[0].position.row, 2, "row = header_rows = 2");
        assert_eq!(bar_numbers[0].horizontal_alignment, HorizontalAlignment::Left);
        assert_eq!(bar_numbers[0].vertical_alignment, VerticalAlignment::Bottom);
    }

    #[test]
    fn cross_measure_tie_emits_right_half_arc_on_line_wrap() {
        // With default max_columns=28:
        // Measure 1: 4 (directives) + 16 (notes) + 1 (bar) = 21 cols
        // Measure 2: 0 + 16 + 1 = 17 cols → 21+17=38 > 28 → wraps to new line
        // 3~ at col 16 in measure 1 should produce a right-half arc ending at the bar line (col 20).
        let score = make_score("0 0 0 3~ | 3 0 0 0", "a");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let curves = collect_curves(&pages);
        assert!(!curves.is_empty(), "expected right-half tie arc when cross-measure tie wraps to new line");
        // The right-half arc starts at the tied note (col 16) and ends at the bar line (col 20 = 21-1).
        assert!(curves.iter().any(|&(from, to)| from == 16 && to == 20),
            "expected right-half arc from col 16 to col 20; got: {:?}", curves);
    }

    #[test]
    fn cross_measure_tie_continuation_does_not_consume_lyric_on_line_wrap() {
        // The continuation note (3 in measure 2) must NOT consume a lyric syllable
        // because prev_tie is preserved across the line boundary.
        // Only the 3~ note in measure 1 should consume a lyric.
        let score = make_score("0 0 0 3~ | 3 0 0 0", "a b");
        let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
        let lyrics = collect_lyric_positions(&pages);
        assert_eq!(lyrics.len(), 1,
            "continuation note across line break must not consume a lyric syllable; got: {:?}", lyrics);
        assert_eq!(lyrics[0].1, "a");
    }
}
