use crate::ast::parsed::{
    flatten_score_line_slots, ParsedLyrics, ParsedScore, ParsedTimedTrack, ParsedTrack, PartDecl,
    PartKind, ScoreEvent, ScoreLineRole, ScoreLineSlot,
};
use crate::error::{JianPuError, Span, Spanned};
use crate::parser::score::token_parser::{self, GroupStack};
use crate::utils::{count_lyric_slots_in_events, tokenize_lyrics, LyricTieState};

#[path = "interleaved_beat_padding.rs"]
mod beat_padding;
#[path = "interleaved_directives.rs"]
mod directives;

use beat_padding::{beats_per_measure, validate_and_pad_beats};
use directives::{collect_groups, split_directive};

enum SlotAction {
    Chord { track_index: usize },
    Notes { track_index: usize },
    Lyrics { track_index: usize },
}

enum TrackAccumulator {
    Timed {
        events: Vec<Spanned<ScoreEvent>>,
        syllables: Option<Vec<crate::ast::parsed::Syllable>>,
    },
}

struct BarGroupContext<'a> {
    base_offset: usize,
    declarations: &'a [PartDecl],
    slots: &'a [ScoreLineSlot],
    slot_actions: &'a [SlotAction],
    first_notes_track_index: usize,
    time_num: &'a mut u8,
    time_den: &'a mut u8,
    accumulators: &'a mut [TrackAccumulator],
    lyric_tie_states: &'a mut [LyricTieState],
    group_states: &'a mut [GroupStack],
    bar_lyric_slots: &'a mut [Option<u32>],
}

pub fn parse(
    content: &str,
    base_offset: usize,
    declarations: &[PartDecl],
) -> Result<Vec<ParsedTrack>, JianPuError> {
    let groups = collect_groups(content);
    let groups = crate::desugar::desugar_groups(groups, declarations)?;

    let first_notes_track_index = declarations
        .iter()
        .position(|d| matches!(d.kind, PartKind::Notes | PartKind::NotesWithLyrics))
        .ok_or_else(|| {
            JianPuError::new(
                Span::new(base_offset, base_offset + content.len()),
                "parts declaration has no notes track",
            )
        })?;

    let slots = flatten_score_line_slots(declarations);
    let slot_actions = build_slot_actions(&slots);
    let mut accumulators = init_accumulators(declarations);

    let mut time_num: u8 = 4;
    let mut time_den: u8 = 4;
    let mut lyric_tie_states = vec![LyricTieState::default(); declarations.len()];
    let mut group_states = vec![GroupStack::default(); declarations.len()];
    let mut bar_lyric_slots = vec![None; declarations.len()];

    let mut ctx = BarGroupContext {
        base_offset,
        declarations,
        slots: &slots,
        slot_actions: &slot_actions,
        first_notes_track_index,
        time_num: &mut time_num,
        time_den: &mut time_den,
        accumulators: &mut accumulators,
        lyric_tie_states: &mut lyric_tie_states,
        group_states: &mut group_states,
        bar_lyric_slots: &mut bar_lyric_slots,
    };

    for (bar_idx, group_lines) in groups.iter().enumerate() {
        process_bar_group(group_lines, bar_idx + 1, &mut ctx)?;
    }

    for (track_index, state) in group_states.iter().enumerate() {
        if state.is_open() {
            let part_label = declarations
                .get(track_index)
                .map(|d| d.abbreviation.as_str())
                .unwrap_or("unknown");
            return Err(JianPuError::new(
                Span::new(base_offset, base_offset + content.len()),
                format!("unclosed '(' group at end of score in part '{part_label}'"),
            ));
        }
    }

    build_parse_result(declarations, accumulators)
}

fn build_slot_actions(slots: &[ScoreLineSlot]) -> Vec<SlotAction> {
    slots
        .iter()
        .map(|slot| match slot.role {
            ScoreLineRole::Chord => SlotAction::Chord {
                track_index: slot.track_index,
            },
            ScoreLineRole::Notes => SlotAction::Notes {
                track_index: slot.track_index,
            },
            ScoreLineRole::Lyrics => SlotAction::Lyrics {
                track_index: slot.track_index,
            },
        })
        .collect()
}

fn init_accumulators(declarations: &[PartDecl]) -> Vec<TrackAccumulator> {
    declarations
        .iter()
        .map(|decl| TrackAccumulator::Timed {
            events: Vec::new(),
            syllables: if matches!(decl.kind, PartKind::NotesWithLyrics) {
                Some(Vec::new())
            } else {
                None
            },
        })
        .collect()
}

fn process_bar_group(
    group_lines: &[(String, usize)],
    bar: usize,
    ctx: &mut BarGroupContext<'_>,
) -> Result<(), JianPuError> {
    let (directive_events, data_lines) = split_directive(group_lines, bar)?;

    for e in &directive_events {
        if let ScoreEvent::TimeSignatureChange {
            numerator,
            denominator,
        } = &e.value
        {
            *ctx.time_num = *numerator;
            *ctx.time_den = *denominator;
        }
    }

    let padded_data =
        validate_and_pad_group_lines(group_lines, data_lines, ctx.slots, ctx.base_offset)?;

    for slot in ctx.bar_lyric_slots.iter_mut() {
        *slot = None;
    }

    if !directive_events.is_empty() {
        let events_acc = timed_events_mut(
            ctx.accumulators
                .get_mut(ctx.first_notes_track_index)
                .ok_or_else(|| {
                    JianPuError::new(
                        Span::new(ctx.base_offset, ctx.base_offset + 1),
                        "internal error: missing notes accumulator for directive events",
                    )
                })?,
        )?;
        events_acc.extend(directive_events);
    }

    let beats_expected = beats_per_measure(*ctx.time_num, *ctx.time_den);
    process_padded_columns(&padded_data, bar, beats_expected, ctx)
}

fn timed_events_mut(
    acc: &mut TrackAccumulator,
) -> Result<&mut Vec<Spanned<ScoreEvent>>, JianPuError> {
    match acc {
        TrackAccumulator::Timed { events, .. } => Ok(events),
    }
}

fn notes_syllables_mut(
    acc: &mut TrackAccumulator,
) -> Result<Option<&mut Vec<crate::ast::parsed::Syllable>>, JianPuError> {
    match acc {
        TrackAccumulator::Timed { syllables, .. } => Ok(syllables.as_mut()),
    }
}

fn process_padded_columns(
    padded_data: &[(String, usize)],
    bar: usize,
    beats_expected: u32,
    ctx: &mut BarGroupContext<'_>,
) -> Result<(), JianPuError> {
    for (i, (line, line_offset)) in padded_data.iter().enumerate() {
        process_column_line(i, line, *line_offset, bar, beats_expected, ctx)?;
    }
    Ok(())
}

fn validate_lyrics_syllable_count(
    bar: usize,
    track_index: usize,
    syllable_count: usize,
    line_span: Span,
    ctx: &BarGroupContext<'_>,
) -> Result<(), JianPuError> {
    let Some(expected_slots) = ctx.bar_lyric_slots.get(track_index).and_then(|s| *s) else {
        return Ok(());
    };
    let expected = expected_slots as usize;
    if syllable_count == expected {
        return Ok(());
    }
    let part_label = ctx
        .declarations
        .get(track_index)
        .map(|d| d.abbreviation.as_str())
        .unwrap_or("unknown");
    Err(JianPuError::new(
        line_span,
        format!(
            "bar {bar}: lyrics has {syllable_count} syllable{} but notes need {expected} in part '{part_label}'",
            if syllable_count == 1 { "" } else { "s" }
        ),
    ))
}

fn process_lyrics_column_line(
    track_index: usize,
    line: &str,
    line_span: Span,
    bar: usize,
    ctx: &mut BarGroupContext<'_>,
) -> Result<(), JianPuError> {
    if line.is_empty() {
        return Err(JianPuError::new(
            line_span,
            "lyrics line cannot be empty; use '_' for no lyrics".to_string(),
        ));
    }
    if line == "_" {
        return Ok(());
    }
    let syllables = tokenize_lyrics(line);
    validate_lyrics_syllable_count(bar, track_index, syllables.len(), line_span.clone(), ctx)?;
    let acc = ctx.accumulators.get_mut(track_index).ok_or_else(|| {
        JianPuError::new(
            line_span.clone(),
            "internal error: track accumulator index out of range",
        )
    })?;
    let Some(syllables_acc) = notes_syllables_mut(acc)? else {
        let abbrev = ctx
            .declarations
            .get(track_index)
            .map(|d| d.abbreviation.as_str())
            .unwrap_or("unknown");
        return Err(JianPuError::new(
            line_span,
            format!("lyrics line for '{abbrev}' has no matching notes track"),
        ));
    };
    syllables_acc.extend(syllables);
    Ok(())
}

fn process_column_line(
    slot_idx: usize,
    line: &str,
    line_offset: usize,
    bar: usize,
    beats_expected: u32,
    ctx: &mut BarGroupContext<'_>,
) -> Result<(), JianPuError> {
    let line_span = Span::new(
        ctx.base_offset + line_offset,
        ctx.base_offset + line_offset + line.len(),
    );
    let slot_action = ctx.slot_actions.get(slot_idx).ok_or_else(|| {
        JianPuError::new(line_span.clone(), "internal error: slot index out of range")
    })?;
    match slot_action {
        SlotAction::Notes { track_index } => {
            if line == "_" {
                return Err(JianPuError::new(
                    line_span,
                    "'_' is only valid on lyrics lines; use '-' for rests in notes".to_string(),
                ));
            }
            let group_state = ctx.group_states.get_mut(*track_index).ok_or_else(|| {
                JianPuError::new(
                    line_span.clone(),
                    "internal error: group state index out of range",
                )
            })?;
            let events = validate_and_pad_beats(
                token_parser::parse_notes_line(line, ctx.base_offset + line_offset, group_state)?,
                beats_expected,
                *ctx.time_num,
                *ctx.time_den,
            )?;
            if let Some(tie_state) = ctx.lyric_tie_states.get_mut(*track_index) {
                let slots = count_lyric_slots_in_events(&events, tie_state);
                if let Some(bar_slot) = ctx.bar_lyric_slots.get_mut(*track_index) {
                    *bar_slot = Some(slots);
                }
            }
            let acc = ctx.accumulators.get_mut(*track_index).ok_or_else(|| {
                JianPuError::new(
                    line_span.clone(),
                    "internal error: notes accumulator index out of range",
                )
            })?;
            timed_events_mut(acc)?.extend(events);
        }
        SlotAction::Lyrics { track_index } => {
            process_lyrics_column_line(*track_index, line, line_span, bar, ctx)?;
        }
        SlotAction::Chord { track_index } => {
            if line == "_" {
                return Err(JianPuError::new(
                    line_span,
                    "'_' is only valid on lyrics lines".to_string(),
                ));
            }
            let group_state = ctx.group_states.get_mut(*track_index).ok_or_else(|| {
                JianPuError::new(
                    line_span.clone(),
                    "internal error: group state index out of range",
                )
            })?;
            let events = validate_and_pad_beats(
                token_parser::parse_chord_line(line, ctx.base_offset + line_offset, group_state)?,
                beats_expected,
                *ctx.time_num,
                *ctx.time_den,
            )?;
            let acc = ctx.accumulators.get_mut(*track_index).ok_or_else(|| {
                JianPuError::new(
                    line_span,
                    "internal error: chord accumulator index out of range",
                )
            })?;
            timed_events_mut(acc)?.extend(events);
        }
    }
    Ok(())
}

fn validate_and_pad_group_lines(
    group_lines: &[(String, usize)],
    data_lines: &[(String, usize)],
    slots: &[ScoreLineSlot],
    base_offset: usize,
) -> Result<Vec<(String, usize)>, JianPuError> {
    let group_first_span = group_lines
        .first()
        .map(|(line, off)| Span::new(base_offset + off, base_offset + off + line.len()))
        .unwrap_or_else(|| Span::new(base_offset, base_offset));

    if data_lines.is_empty() {
        return Err(JianPuError::new(
            group_first_span,
            "expected at least one data line in measure group".to_string(),
        ));
    }
    if data_lines.len() != slots.len() {
        return Err(JianPuError::new(
            group_first_span,
            format!(
                "expected {} lines (one per score line), got {}",
                slots.len(),
                data_lines.len()
            ),
        ));
    }

    Ok(data_lines.to_vec())
}

fn build_parse_result(
    declarations: &[PartDecl],
    accumulators: Vec<TrackAccumulator>,
) -> Result<Vec<ParsedTrack>, JianPuError> {
    if declarations.len() != accumulators.len() {
        return Err(JianPuError::new(
            Span::new(0, 0),
            "internal error: declaration/accumulator count mismatch",
        ));
    }

    declarations
        .iter()
        .zip(accumulators)
        .map(|(decl, acc)| {
            let TrackAccumulator::Timed { events, syllables } = acc;
            Ok(ParsedTrack::Timed(ParsedTimedTrack {
                abbreviation: decl.abbreviation.clone(),
                display_name: decl.display_name.clone(),
                kind: decl.kind,
                score: ParsedScore { events },
                lyrics: syllables.map(|s| ParsedLyrics { syllables: s }),
            }))
        })
        .collect()
}

#[cfg(test)]
#[path = "interleaved_parser_test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
#[path = "interleaved_parser_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "interleaved_parser_padding_tests.rs"]
mod padding_tests;
