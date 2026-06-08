use crate::ast::parsed::{
    Accidental, KeyChange, Note, NoteName, ParsedChordEvent, ParsedChordPart, ParsedLyrics,
    ParsedPart, ParsedScore, PartColumn, ScoreEvent,
};
use crate::error::{JianPuError, Span, Spanned};
use crate::parser::score::{token_parser, tokenizer};
use crate::utils::{count_lyric_slots_in_events, tokenize_lyrics, LyricTieState};

enum ColAction {
    Notes(usize),
    Lyrics(usize),
    Chord(usize),
}

struct ParseAccumulators {
    events_acc: Vec<Vec<Spanned<ScoreEvent>>>,
    syllables_acc: Vec<Option<Vec<crate::ast::parsed::Syllable>>>,
    chord_events_acc: Vec<Vec<Vec<ParsedChordEvent>>>,
}

struct BarGroupContext<'a> {
    base_offset: usize,
    parts: &'a [PartColumn],
    notes_names: &'a [String],
    col_actions: &'a [ColAction],
    time_num: &'a mut u8,
    time_den: &'a mut u8,
    accumulators: &'a mut ParseAccumulators,
    lyric_tie_states: &'a mut [LyricTieState],
    bar_lyric_slots: &'a mut [Option<u32>],
}

pub fn parse(
    content: &str,
    base_offset: usize,
    parts: &[PartColumn],
) -> Result<(Vec<ParsedPart>, Vec<ParsedChordPart>), JianPuError> {
    let groups = collect_groups(content);
    let groups = crate::desugar::desugar_groups(groups, parts)?;

    let notes_names: Vec<String> = parts
        .iter()
        .filter_map(|p| match p {
            PartColumn::Notes { name } => Some(name.clone()),
            _ => None,
        })
        .collect();

    if notes_names.is_empty() {
        return Err(JianPuError::new(
            Span::new(base_offset, base_offset + content.len()),
            "parts declaration has no 'notes:' columns",
        ));
    }

    let chord_names: Vec<String> = parts
        .iter()
        .filter_map(|p| match p {
            PartColumn::Chord { name } => Some(name.clone()),
            _ => None,
        })
        .collect();

    let col_actions = build_col_actions(parts, &notes_names, base_offset, content.len())?;
    let mut accumulators = init_accumulators(&notes_names, &chord_names, parts);

    let mut time_num: u8 = 4;
    let mut time_den: u8 = 4;
    let mut lyric_tie_states = vec![LyricTieState::default(); notes_names.len()];
    let mut bar_lyric_slots = vec![None; notes_names.len()];

    let mut ctx = BarGroupContext {
        base_offset,
        parts,
        notes_names: &notes_names,
        col_actions: &col_actions,
        time_num: &mut time_num,
        time_den: &mut time_den,
        accumulators: &mut accumulators,
        lyric_tie_states: &mut lyric_tie_states,
        bar_lyric_slots: &mut bar_lyric_slots,
    };

    for (bar_idx, group_lines) in groups.iter().enumerate() {
        process_bar_group(group_lines, bar_idx + 1, &mut ctx)?;
    }

    build_parse_result(
        base_offset,
        content.len(),
        &notes_names,
        chord_names,
        accumulators,
    )
}

fn build_col_actions(
    parts: &[PartColumn],
    notes_names: &[String],
    base_offset: usize,
    content_len: usize,
) -> Result<Vec<ColAction>, JianPuError> {
    let mut chord_name_idx = 0usize;
    let mut col_actions = Vec::with_capacity(parts.len());
    let parts_span = Span::new(base_offset, base_offset + content_len);

    for p in parts {
        match p {
            PartColumn::Notes { name } => {
                let idx = notes_names.iter().position(|n| n == name).ok_or_else(|| {
                    JianPuError::new(
                        parts_span.clone(),
                        format!("notes column '{name}' is not declared in parts"),
                    )
                })?;
                col_actions.push(ColAction::Notes(idx));
            }
            PartColumn::Lyrics { name } => {
                let idx = notes_names.iter().position(|n| n == name).ok_or_else(|| {
                    JianPuError::new(
                        parts_span.clone(),
                        format!("lyrics column '{name}' has no matching notes column"),
                    )
                })?;
                col_actions.push(ColAction::Lyrics(idx));
            }
            PartColumn::Chord { .. } => {
                col_actions.push(ColAction::Chord(chord_name_idx));
                chord_name_idx += 1;
            }
        }
    }

    Ok(col_actions)
}

fn init_accumulators(
    notes_names: &[String],
    chord_names: &[String],
    parts: &[PartColumn],
) -> ParseAccumulators {
    let mut syllables_acc: Vec<Option<Vec<crate::ast::parsed::Syllable>>> =
        (0..notes_names.len()).map(|_| None).collect();

    for p in parts {
        if let PartColumn::Lyrics { name } = p {
            if let Some(idx) = notes_names.iter().position(|n| n == name) {
                if let Some(slot) = syllables_acc.get_mut(idx) {
                    *slot = Some(Vec::new());
                }
            }
        }
    }

    ParseAccumulators {
        events_acc: (0..notes_names.len()).map(|_| Vec::new()).collect(),
        syllables_acc,
        chord_events_acc: (0..chord_names.len()).map(|_| Vec::new()).collect(),
    }
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
        validate_and_pad_group_lines(group_lines, data_lines, ctx.parts, ctx.base_offset)?;

    for slot in ctx.bar_lyric_slots.iter_mut() {
        *slot = None;
    }

    if !directive_events.is_empty() {
        let events_acc = ctx.accumulators.events_acc.get_mut(0).ok_or_else(|| {
            JianPuError::new(
                Span::new(ctx.base_offset, ctx.base_offset + 1),
                "internal error: missing notes accumulator for directive events",
            )
        })?;
        events_acc.extend(directive_events);
    }

    let beats_expected = beats_per_measure(*ctx.time_num, *ctx.time_den);
    process_padded_columns(&padded_data, bar, beats_expected, ctx)
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
    part_idx: usize,
    syllable_count: usize,
    line_span: Span,
    ctx: &BarGroupContext<'_>,
) -> Result<(), JianPuError> {
    let Some(expected_slots) = ctx.bar_lyric_slots.get(part_idx).and_then(|s| *s) else {
        return Ok(());
    };
    let expected = expected_slots as usize;
    if syllable_count == expected {
        return Ok(());
    }
    let part_label = ctx
        .notes_names
        .get(part_idx)
        .filter(|name| !name.is_empty())
        .map(|name| name.as_str())
        .unwrap_or("unnamed part");
    Err(JianPuError::new(
        line_span,
        format!(
            "bar {bar}: lyrics has {syllable_count} syllable{} but notes need {expected} in part '{part_label}'",
            if syllable_count == 1 { "" } else { "s" }
        ),
    ))
}

fn process_lyrics_column_line(
    part_idx: usize,
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
    validate_lyrics_syllable_count(bar, part_idx, syllables.len(), line_span.clone(), ctx)?;
    let Some(acc) = ctx
        .accumulators
        .syllables_acc
        .get_mut(part_idx)
        .and_then(|slot| slot.as_mut())
    else {
        return Err(JianPuError::new(
            line_span,
            "lyrics column has no matching notes column",
        ));
    };
    acc.extend(syllables);
    Ok(())
}

fn process_column_line(
    col_idx: usize,
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
    let col_action = ctx.col_actions.get(col_idx).ok_or_else(|| {
        JianPuError::new(
            line_span.clone(),
            "internal error: column index out of range",
        )
    })?;
    match col_action {
        ColAction::Notes(idx) => {
            if line == "_" {
                return Err(JianPuError::new(
                    line_span,
                    "'_' is only valid on lyrics lines; use '-' for rests in notes".to_string(),
                ));
            }
            let tokens = tokenizer::tokenize(line, ctx.base_offset + line_offset);
            let events = token_parser::parse_tokens(tokens)?;
            validate_beats(&events, beats_expected, bar)?;
            if let Some(tie_state) = ctx.lyric_tie_states.get_mut(*idx) {
                let slots = count_lyric_slots_in_events(&events, tie_state);
                if let Some(bar_slot) = ctx.bar_lyric_slots.get_mut(*idx) {
                    *bar_slot = Some(slots);
                }
            }
            let events_acc = ctx.accumulators.events_acc.get_mut(*idx).ok_or_else(|| {
                JianPuError::new(
                    line_span.clone(),
                    "internal error: notes accumulator index out of range",
                )
            })?;
            events_acc.extend(events);
        }
        ColAction::Lyrics(idx) => {
            process_lyrics_column_line(*idx, line, line_span, bar, ctx)?;
        }
        ColAction::Chord(chord_idx) => {
            if line == "_" {
                return Err(JianPuError::new(
                    line_span,
                    "'_' is only valid on lyrics lines".to_string(),
                ));
            }
            let events =
                crate::parser::score::chord_parser::parse(line, ctx.base_offset + line_offset)?;
            let chord_acc = ctx
                .accumulators
                .chord_events_acc
                .get_mut(*chord_idx)
                .ok_or_else(|| {
                    JianPuError::new(
                        line_span,
                        "internal error: chord accumulator index out of range",
                    )
                })?;
            chord_acc.push(events);
        }
    }
    Ok(())
}

fn validate_and_pad_group_lines(
    group_lines: &[(String, usize)],
    data_lines: &[(String, usize)],
    parts: &[PartColumn],
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
    if data_lines.len() > parts.len() {
        return Err(JianPuError::new(
            group_first_span,
            format!(
                "expected at most {} lines (one per parts column), got {}",
                parts.len(),
                data_lines.len()
            ),
        ));
    }
    if data_lines.len() < parts.len() {
        return Err(JianPuError::new(
            group_first_span,
            format!(
                "expected {} lines (one per parts column), got {}",
                parts.len(),
                data_lines.len()
            ),
        ));
    }

    Ok(data_lines.to_vec())
}

fn build_parse_result(
    base_offset: usize,
    content_len: usize,
    notes_names: &[String],
    chord_names: Vec<String>,
    mut accumulators: ParseAccumulators,
) -> Result<(Vec<ParsedPart>, Vec<ParsedChordPart>), JianPuError> {
    let internal_span = Span::new(base_offset, base_offset + content_len);
    let mut notes_result = Vec::new();
    for (i, name) in notes_names.iter().enumerate() {
        let events = accumulators
            .events_acc
            .get_mut(i)
            .ok_or_else(|| {
                JianPuError::new(
                    internal_span.clone(),
                    "internal error: notes accumulator index out of range",
                )
            })
            .map(std::mem::take)?;
        let lyrics = accumulators
            .syllables_acc
            .get_mut(i)
            .ok_or_else(|| {
                JianPuError::new(
                    internal_span.clone(),
                    "internal error: lyrics accumulator index out of range",
                )
            })?
            .take()
            .map(|s| ParsedLyrics { syllables: s });
        notes_result.push(ParsedPart {
            name: if name.is_empty() {
                None
            } else {
                Some(name.clone())
            },
            score: ParsedScore { events },
            lyrics,
        });
    }

    let chord_parts: Vec<ParsedChordPart> = chord_names
        .into_iter()
        .zip(accumulators.chord_events_acc)
        .map(|(name, events_per_measure)| ParsedChordPart {
            name: if name.is_empty() { None } else { Some(name) },
            events_per_measure,
        })
        .collect();

    Ok((notes_result, chord_parts))
}

/// Returns groups of `(trimmed_line, byte_offset_within_content)` pairs.
fn collect_groups(content: &str) -> Vec<Vec<(String, usize)>> {
    let mut groups: Vec<Vec<(String, usize)>> = Vec::new();
    let mut current: Vec<(String, usize)> = Vec::new();
    let mut byte_offset: usize = 0;

    for line in content.lines() {
        let leading = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
        } else {
            current.push((trimmed.to_string(), byte_offset + leading));
        }
        byte_offset += line.len() + 1; // +1 for '\n'
    }
    if !current.is_empty() {
        groups.push(current);
    }

    groups
}

#[allow(clippy::type_complexity)]
fn split_directive(
    lines: &[(String, usize)],
    _bar: usize,
) -> Result<(Vec<Spanned<ScoreEvent>>, &[(String, usize)]), JianPuError> {
    if let Some((directive_line, directive_offset)) = lines.first() {
        if directive_line.starts_with('(') {
            if !directive_line.ends_with(')') {
                return Err(JianPuError::new(
                    Span::new(*directive_offset, directive_offset + directive_line.len()),
                    "directive row must end with ')'",
                ));
            }
            let events = parse_directive_line(directive_line, *directive_offset)?;
            let remaining = lines.get(1..).unwrap_or(&[]);
            return Ok((events, remaining));
        }
    }
    Ok((Vec::new(), lines))
}

/// Returns `(token_text, byte_offset_within_inner)` pairs.
fn tokenize_directive_tokens(inner: &str) -> Result<Vec<(String, usize)>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_start: usize = 0;
    let mut in_quote = false;
    let mut byte_offset: usize = 0;

    for ch in inner.chars() {
        if in_quote {
            current.push(ch);
            if ch == '"' {
                in_quote = false;
            }
            byte_offset += ch.len_utf8();
        } else if ch == '"' {
            if current.is_empty() {
                current_start = byte_offset;
            }
            current.push(ch);
            in_quote = true;
            byte_offset += ch.len_utf8();
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push((std::mem::take(&mut current), current_start));
            }
            byte_offset += ch.len_utf8();
        } else {
            if current.is_empty() {
                current_start = byte_offset;
            }
            current.push(ch);
            byte_offset += ch.len_utf8();
        }
    }
    if in_quote {
        return Err("unclosed quote in directive line".to_string());
    }
    if !current.is_empty() {
        tokens.push((current, current_start));
    }
    Ok(tokens)
}

fn parse_directive_line(
    line: &str,
    line_offset: usize,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    let inner = &line[1..line.len() - 1];
    let inner_offset = line_offset + 1; // skip '('
    let tokens = tokenize_directive_tokens(inner)
        .map_err(|msg| JianPuError::new(Span::new(line_offset, line_offset + line.len()), msg))?;
    let mut events = Vec::new();

    for (token, token_inner_offset) in &tokens {
        let token_file_offset = inner_offset + token_inner_offset;
        let span = Span::new(token_file_offset, token_file_offset + token.len());

        let event = if let Some(rest) = token.strip_prefix("bpm=") {
            let bpm = rest.parse::<u32>().map_err(|_| {
                JianPuError::new(span.clone(), format!("invalid bpm value: {rest}"))
            })?;
            ScoreEvent::BpmChange(bpm)
        } else if let Some(rest) = token.strip_prefix("key=") {
            parse_key_value(rest, span.clone())?
        } else if let Some(rest) = token.strip_prefix("time=") {
            parse_time_value(rest, span.clone())?
        } else if let Some(rest) = token.strip_prefix("label=") {
            if rest.len() < 2 || !rest.starts_with('"') || !rest.ends_with('"') {
                return Err(JianPuError::new(
                    span,
                    format!("label value must be a quoted string, got: {rest}"),
                ));
            }
            let text = rest[1..rest.len() - 1].to_string();
            if text.is_empty() {
                return Err(JianPuError::new(
                    span,
                    "label value must not be empty".to_string(),
                ));
            }
            ScoreEvent::LabelChange(text)
        } else {
            return Err(JianPuError::new(
                span,
                format!("unknown directive: '{token}'"),
            ));
        };

        events.push(Spanned::new(event, span));
    }

    Ok(events)
}

fn parse_key_value(value: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    let mut chars = value.chars().peekable();

    let name_char = chars.next().ok_or_else(|| {
        JianPuError::new(span.clone(), "expected note name after 'key='".to_string())
    })?;

    let name = match name_char {
        'A' => NoteName::A,
        'B' => NoteName::B,
        'C' => NoteName::C,
        'D' => NoteName::D,
        'E' => NoteName::E,
        'F' => NoteName::F,
        'G' => NoteName::G,
        _ => {
            return Err(JianPuError::new(
                span,
                format!("invalid note name: '{name_char}'"),
            ))
        }
    };

    let accidental = match chars.peek() {
        Some('b') => {
            chars.next();
            Accidental::Flat
        }
        Some('#') => {
            chars.next();
            Accidental::Sharp
        }
        _ => Accidental::Natural,
    };

    let octave_str: String = chars.collect();
    let octave = octave_str.parse::<u8>().map_err(|_| {
        JianPuError::new(
            span.clone(),
            format!("invalid octave in 'key={value}': expected number"),
        )
    })?;

    Ok(ScoreEvent::KeyChange(KeyChange {
        note: Note {
            name,
            octave,
            accidental,
        },
    }))
}

fn parse_time_value(value: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() != 2 {
        return Err(JianPuError::new(
            span,
            format!("invalid time signature: '{value}'"),
        ));
    }
    let numerator_str = parts.first().ok_or_else(|| {
        JianPuError::new(span.clone(), format!("invalid time signature: '{value}'"))
    })?;
    let denominator_str = parts.get(1).ok_or_else(|| {
        JianPuError::new(span.clone(), format!("invalid time signature: '{value}'"))
    })?;
    let numerator = numerator_str.parse::<u8>().map_err(|_| {
        JianPuError::new(
            span.clone(),
            format!("invalid time numerator: '{numerator_str}'"),
        )
    })?;
    let denominator = denominator_str.parse::<u8>().map_err(|_| {
        JianPuError::new(
            span.clone(),
            format!("invalid time denominator: '{denominator_str}'"),
        )
    })?;
    if denominator == 0 {
        return Err(JianPuError::new(
            span,
            "time denominator cannot be zero".to_string(),
        ));
    }
    Ok(ScoreEvent::TimeSignatureChange {
        numerator,
        denominator,
    })
}

fn beats_per_measure(num: u8, den: u8) -> u32 {
    (num as u32) * (16 / den as u32)
}

fn validate_beats(
    events: &[Spanned<ScoreEvent>],
    expected: u32,
    _bar: usize,
) -> Result<(), JianPuError> {
    let mut total = 0u32;

    for e in events {
        let beats = match &e.value {
            ScoreEvent::Note(n) => n.duration,
            ScoreEvent::Rest(r) => r.duration,
            ScoreEvent::Extension => 4,
            _ => 0,
        };
        if beats > 0 {
            total += beats;
            if total > expected {
                return Err(JianPuError::new(
                    e.span.clone(),
                    format!(
                        "note exceeds measure boundary: measure has {expected} quarter-beats, cumulative is now {total}"
                    ),
                ));
            }
        }
    }

    if total < expected {
        let span = events
            .iter()
            .rfind(|e| {
                matches!(
                    &e.value,
                    ScoreEvent::Note(_) | ScoreEvent::Rest(_) | ScoreEvent::Extension
                )
            })
            .map(|e| e.span.clone())
            // structurally unreachable: a data line always has at least one token
            .unwrap_or(Span::new(0, 1));
        return Err(JianPuError::new(
            span,
            format!("incomplete measure: expected {expected} quarter-beats, got {total}"),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parsed::PartColumn;

    fn notes_col(name: &str) -> PartColumn {
        PartColumn::Notes {
            name: name.to_string(),
        }
    }
    fn lyrics_col(name: &str) -> PartColumn {
        PartColumn::Lyrics {
            name: name.to_string(),
        }
    }
    fn chord_col(name: &str) -> PartColumn {
        PartColumn::Chord {
            name: name.to_string(),
        }
    }

    #[test]
    fn chord_column_events_are_parsed() {
        use crate::ast::parsed::{
            Accidental, JianPuPitch, ParsedChordEvent, ParsedChordSymbol, PartColumn, TriadQuality,
        };
        let parts = vec![
            PartColumn::Chord {
                name: "main".to_string(),
            },
            PartColumn::Notes {
                name: "main".to_string(),
            },
        ];
        let content = "(time=4/4 key=C4 bpm=120)\n1 - - -\n1 - - -\n";
        let (note_parts, chord_parts) = parse(content, 0, &parts).unwrap();
        assert_eq!(chord_parts.len(), 1);
        assert_eq!(chord_parts[0].events_per_measure.len(), 1);
        let events = &chord_parts[0].events_per_measure[0];
        assert_eq!(
            events[0],
            ParsedChordEvent::Chord(ParsedChordSymbol {
                degree: JianPuPitch::One,
                accidental: Accidental::Natural,
                triad: TriadQuality::Major,
                extension: None,
                bass: None,
            })
        );
        assert!(matches!(events[1], ParsedChordEvent::Extend(_)));
        // note_parts should contain the notes column
        assert_eq!(note_parts.len(), 1);
    }

    #[test]
    fn single_unnamed_part_no_lyrics() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, None);
        assert!(result[0].lyrics.is_none());
        assert_eq!(result[0].score.events.len(), 7);
    }

    #[test]
    fn single_part_with_lyrics() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\ndo re mi fa\n";
        let parts = vec![notes_col(""), lyrics_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].lyrics.is_some());
        assert_eq!(result[0].lyrics.as_ref().unwrap().syllables.len(), 4);
    }

    #[test]
    fn two_parts_two_bars() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
            "\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
        );
        let parts = vec![notes_col("Soprano"), notes_col("Alto")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, Some("Soprano".to_string()));
        assert_eq!(result[1].name, Some("Alto".to_string()));
        assert_eq!(result[0].score.events.len(), 11);
        assert_eq!(result[1].score.events.len(), 8);
    }

    #[test]
    fn rejects_too_many_lines_in_group() {
        // 3 lines for 2-column parts declaration → error
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\nextra line\n";
        let parts = vec![notes_col(""), lyrics_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert!(err.message.contains("expected") && err.message.contains("got"));
    }

    #[test]
    fn underscore_on_lyrics_line_means_no_lyrics_for_that_bar() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
            "\n",
            "5 6 7 1\n",
            "_\n",
        );
        let parts = vec![notes_col(""), lyrics_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].lyrics.as_ref().unwrap().syllables.len(), 4);
    }

    #[test]
    fn rejects_too_few_lyrics_syllables_for_notes() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c\n";
        let parts = vec![notes_col(""), lyrics_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert!(
            err.message
                .contains("lyrics has 3 syllables but notes need 4"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn rejects_too_many_lyrics_syllables_for_notes() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d e\n";
        let parts = vec![notes_col(""), lyrics_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert!(
            err.message
                .contains("lyrics has 5 syllables but notes need 4"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn tied_notes_share_one_lyric_slot_in_bar() {
        let content = "(time=4/4 key=C4 bpm=120)\n3~3 1 2\na b c\n";
        let parts = vec![notes_col(""), lyrics_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert_eq!(result[0].lyrics.as_ref().unwrap().syllables.len(), 3);
    }

    #[test]
    fn cross_measure_tie_continuation_needs_fewer_lyrics() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n0 0 0 3~\na\n",
            "\n",
            "3 0 0 0\n",
            "_\n",
        );
        let parts = vec![notes_col(""), lyrics_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert_eq!(result[0].lyrics.as_ref().unwrap().syllables.len(), 1);
    }

    #[test]
    fn rejects_omitted_trailing_lyrics_without_precedent() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
            "\n",
            "5 6 7 1\n",
        );
        let parts = vec![notes_col(""), lyrics_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert!(
            err.message.contains("expected lyrics line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn partial_measure_still_needs_ditto_before_diverging_middle_columns() {
        // A2 lyrics ditto must stay explicit when S1/S2 lines follow with real content.
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n",
            "1 - 6m -\n",
            "6* =6 =6 _6 _5 =3 =2~_2\n",
            "a b c d e f g\n",
            "4* =4 =4 _4 _3 =1 =2~_2\n",
            "\"\n",
            "6 - 5 -\n",
            "alto lyrics\n",
        );
        let parts = vec![
            chord_col("main"),
            notes_col("A1"),
            lyrics_col("A1"),
            notes_col("A2"),
            lyrics_col("A2"),
            notes_col("S1"),
            lyrics_col("S1"),
            notes_col("S2"),
            lyrics_col("S2"),
        ];
        let (result, _) = parse(content, 0, &parts).unwrap();
        let s1 = result
            .iter()
            .find(|p| p.name.as_deref() == Some("S1"))
            .unwrap();
        assert_eq!(s1.lyrics.as_ref().unwrap().syllables[0].text, "alto");
    }

    #[test]
    fn implicit_trailing_ditto_matches_explicit_ditto() {
        let explicit = concat!(
            "(time=4/4 key=C4 bpm=120)\n",
            "1 - - -\n",
            "1 2 3 4\n",
            "do re mi fa\n",
            "\"\n",
            "\"\n",
        );
        let implicit = concat!(
            "(time=4/4 key=C4 bpm=120)\n",
            "1 - - -\n",
            "1 2 3 4\n",
            "do re mi fa\n",
        );
        let parts = vec![
            chord_col("main"),
            notes_col("A"),
            lyrics_col("A"),
            notes_col("B"),
            lyrics_col("B"),
        ];
        let (explicit_result, _) = parse(explicit, 0, &parts).unwrap();
        let (implicit_result, _) = parse(implicit, 0, &parts).unwrap();
        assert_eq!(
            explicit_result[0].score.events.len(),
            implicit_result[0].score.events.len()
        );
        assert_eq!(
            explicit_result[1].score.events.len(),
            implicit_result[1].score.events.len()
        );
        assert_eq!(
            explicit_result[0].lyrics.as_ref().unwrap().syllables.len(),
            implicit_result[0].lyrics.as_ref().unwrap().syllables.len()
        );
        assert_eq!(
            explicit_result[1].lyrics.as_ref().unwrap().syllables.len(),
            implicit_result[1].lyrics.as_ref().unwrap().syllables.len()
        );
    }

    #[test]
    fn rejects_overfull_measure() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4 5\n";
        let parts = vec![notes_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert!(err.message.contains("note exceeds measure boundary"));
    }

    #[test]
    fn overfull_measure_span_points_to_offending_note() {
        // "(time=4/4 key=C4 bpm=120)\n" = 26 bytes
        // "1 2 3 4 5" → '5' is at byte offset 8 within the line
        // global offset: 26 + 8 = 34
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4 5\n";
        let parts = vec![notes_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert_eq!(err.span.start, 34, "span must point to the offending '5'");
        assert_eq!(err.span.end, 35);
    }

    #[test]
    fn rejects_underfull_measure() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3\n";
        let parts = vec![notes_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert!(err.message.contains("incomplete measure"));
    }

    #[test]
    fn underfull_measure_span_points_to_last_note() {
        // "(time=4/4 key=C4 bpm=120)\n" = 26 bytes
        // "4 4 4 _4" → '_4' starts at byte offset 6 within the line
        // global offset: 26 + 6 = 32
        let content = "(time=4/4 key=C4 bpm=120)\n4 4 4 _4\n";
        let parts = vec![notes_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert_eq!(err.span.start, 32, "span must point to the last note '_4'");
        assert_eq!(err.span.end, 34);
    }

    #[test]
    fn underfull_measure_in_second_bar_span_points_to_last_note() {
        // "(time=4/4 key=C4 bpm=120)\n" = 26 bytes
        // "5 5 5 5\n"                   =  8 bytes
        // "\n"                          =  1 byte  (blank separator)
        // "4 4 4 _4" → '_4' at offset 6 within line → global 26+8+1+6 = 41
        let content = "(time=4/4 key=C4 bpm=120)\n5 5 5 5\n\n4 4 4 _4\n";
        let parts = vec![notes_col("")];
        let err = parse(content, 0, &parts).unwrap_err();
        assert_eq!(
            err.span.start, 41,
            "span must point to the last note '_4' in the second bar"
        );
        assert_eq!(err.span.end, 43);
    }

    #[test]
    fn directive_row_is_optional() {
        let content = concat!("(time=4/4 key=C4 bpm=120)\n1 2 3 4\n", "\n", "5 6 7 1\n",);
        let parts = vec![notes_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert_eq!(result[0].score.events.len(), 11);
    }

    #[test]
    fn time_sig_change_updates_beat_tracking() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
            "\n",
            "(time=3/4)\n1 2 3\n",
        );
        let parts = vec![notes_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert!(!result[0].score.events.is_empty());
    }

    #[test]
    fn rejects_unknown_directive() {
        let content = "(foo=bar)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        assert!(parse(content, 0, &parts).is_err());
    }

    #[test]
    fn key_directive_parses_flat() {
        let content = "(time=4/4 key=Bb4 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        use crate::ast::parsed::{Accidental, ScoreEvent};
        let key_event = result[0]
            .score
            .events
            .iter()
            .find(|e| matches!(&e.value, ScoreEvent::KeyChange(_)));
        assert!(key_event.is_some());
        if let ScoreEvent::KeyChange(kc) = &key_event.unwrap().value {
            assert_eq!(kc.note.accidental, Accidental::Flat);
        }
    }

    #[test]
    fn label_directive_parsed() {
        let content = "(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        use crate::ast::parsed::ScoreEvent;
        let label_event = result[0]
            .score
            .events
            .iter()
            .find(|e| matches!(&e.value, ScoreEvent::LabelChange(_)));
        assert!(label_event.is_some(), "expected a LabelChange event");
        if let ScoreEvent::LabelChange(text) = &label_event.unwrap().value {
            assert_eq!(text, "Verse 1");
        }
    }

    #[test]
    fn label_directive_rejects_unclosed_quote() {
        let content = "(label=\"Verse 1)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        assert!(parse(content, 0, &parts).is_err());
    }

    #[test]
    fn label_directive_rejects_empty_label() {
        let content = "(label=\"\")\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        assert!(parse(content, 0, &parts).is_err());
    }

    #[test]
    fn notes_ditto_resolves_in_full_parse() {
        // Second part's notes line is `"` — should resolve to first part's notes.
        let content = concat!("(time=4/4 key=C4 bpm=120)\n", "1 2 3 4\n", "\"\n",);
        let parts = vec![notes_col("S"), notes_col("A")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        assert_eq!(result.len(), 2);
        // Soprano gets 3 directive events + 4 notes = 7.
        // Alto gets 4 notes only (directives are added to the first notes part only).
        assert_eq!(result[0].score.events.len(), 7);
        assert_eq!(
            result[1].score.events.len(),
            4,
            "Alto should have 4 note events after ditto resolution"
        );
    }

    #[test]
    fn lyrics_ditto_resolves_in_full_parse() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "do re mi fa\n",
            "\"\n",
            "\"\n",
        );
        let parts = vec![
            notes_col("S"),
            lyrics_col("S"),
            notes_col("A"),
            lyrics_col("A"),
        ];
        let (result, _) = parse(content, 0, &parts).unwrap();
        let s_lyrics = result[0].lyrics.as_ref().unwrap();
        let a_lyrics = result[1].lyrics.as_ref().unwrap();
        assert_eq!(s_lyrics.syllables.len(), 4);
        assert_eq!(a_lyrics.syllables.len(), 4);
        assert_eq!(s_lyrics.syllables[0].text, a_lyrics.syllables[0].text);
    }

    #[test]
    fn key_directive_parses_sharp() {
        let content = "(time=4/4 key=F#3 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let (result, _) = parse(content, 0, &parts).unwrap();
        use crate::ast::parsed::{Accidental, ScoreEvent};
        let key_event = result[0]
            .score
            .events
            .iter()
            .find(|e| matches!(&e.value, ScoreEvent::KeyChange(_)));
        if let ScoreEvent::KeyChange(kc) = &key_event.unwrap().value {
            assert_eq!(kc.note.accidental, Accidental::Sharp);
        }
    }
}
