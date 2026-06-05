use crate::ast::parsed::{
    Accidental, KeyChange, Note, NoteName, ParsedLyrics, ParsedPart, ParsedScore,
    PartColumn, ScoreEvent,
};
use crate::error::{JianPuError, Span, Spanned};
use crate::utils::tokenize_lyrics;
use crate::parser::score::{token_parser, tokenizer};

pub fn parse(content: &str, parts: &[PartColumn]) -> Result<Vec<ParsedPart>, JianPuError> {
    let groups = collect_groups(content);

    let notes_names: Vec<String> = parts.iter().filter_map(|p| match p {
        PartColumn::Notes { name } => Some(name.clone()),
        _ => None,
    }).collect();

    if notes_names.is_empty() {
        return Err(JianPuError::new(Span::new(0, 0), "parts declaration has no 'notes:' columns"));
    }

    enum ColAction { Notes(usize), Lyrics(usize) }

    let col_actions: Vec<ColAction> = parts.iter().map(|p| match p {
        PartColumn::Notes { name } => {
            let idx = notes_names.iter().position(|n| n == name).unwrap();
            ColAction::Notes(idx)
        }
        PartColumn::Lyrics { name } => {
            let idx = notes_names.iter().position(|n| n == name)
                .unwrap_or_else(|| panic!("lyrics column '{}' has no matching notes column", name));
            ColAction::Lyrics(idx)
        }
    }).collect();

    let mut events_acc: Vec<Vec<Spanned<ScoreEvent>>> =
        (0..notes_names.len()).map(|_| Vec::new()).collect();
    let mut syllables_acc: Vec<Option<Vec<crate::ast::parsed::Syllable>>> =
        (0..notes_names.len()).map(|_| None).collect();

    for p in parts {
        if let PartColumn::Lyrics { name } = p {
            if let Some(idx) = notes_names.iter().position(|n| n == name) {
                syllables_acc[idx] = Some(Vec::new());
            }
        }
    }

    let mut time_num: u8 = 4;
    let mut time_den: u8 = 4;

    for (bar_idx, group_lines) in groups.iter().enumerate() {
        let bar = bar_idx + 1;

        let (directive_events, data_lines) = split_directive(group_lines, bar)?;

        for e in &directive_events {
            if let ScoreEvent::TimeSignatureChange { numerator, denominator } = &e.value {
                time_num = *numerator;
                time_den = *denominator;
            }
        }

        // Allow fewer lines than parts only when trailing columns are all Lyrics columns;
        // missing lyrics lines are treated as empty (no syllables).
        // Too many lines or too few notes lines are always errors.
        let notes_cols_count = parts.iter().filter(|p| matches!(p, PartColumn::Notes { .. })).count();
        if data_lines.len() < notes_cols_count {
            return Err(JianPuError::at_bar(bar, 0, format!(
                "expected {} lines (one per parts column), got {}",
                parts.len(), data_lines.len()
            )));
        }
        if data_lines.len() > parts.len() {
            return Err(JianPuError::at_bar(bar, 0, format!(
                "expected {} lines (one per parts column), got {}",
                parts.len(), data_lines.len()
            )));
        }

        // Pad with empty strings for missing trailing lyrics lines
        let padded_data: Vec<String> = (0..parts.len())
            .map(|i| data_lines.get(i).cloned().unwrap_or_default())
            .collect();

        if !directive_events.is_empty() {
            events_acc[0].extend(directive_events);
        }

        let beats_expected = beats_per_measure(time_num, time_den);

        for (i, line) in padded_data.iter().enumerate() {
            match col_actions[i] {
                ColAction::Notes(idx) => {
                    let tokens = tokenizer::tokenize(line, 0);
                    let events = token_parser::parse_tokens(tokens)?;
                    validate_beats(&events, beats_expected, bar)?;
                    events_acc[idx].extend(events);
                }
                ColAction::Lyrics(idx) => {
                    let syllables = tokenize_lyrics(line);
                    syllables_acc[idx].as_mut().unwrap().extend(syllables);
                }
            }
        }
    }

    let mut result = Vec::new();
    for (i, name) in notes_names.iter().enumerate() {
        result.push(ParsedPart {
            name: if name.is_empty() { None } else { Some(name.clone()) },
            score: ParsedScore { events: std::mem::take(&mut events_acc[i]) },
            lyrics: syllables_acc[i].take().map(|s| ParsedLyrics { syllables: s }),
        });
    }

    Ok(result)
}

fn collect_groups(content: &str) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
        } else {
            current.push(trimmed);
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }

    groups
}

fn split_directive(
    lines: &[String],
    bar: usize,
) -> Result<(Vec<Spanned<ScoreEvent>>, &[String]), JianPuError> {
    if lines.first().map(|l| l.starts_with('(')).unwrap_or(false) {
        let directive_line = &lines[0];
        if !directive_line.ends_with(')') {
            return Err(JianPuError::at_bar(bar, 0, "directive row must end with ')'"));
        }
        let events = parse_directive_line(directive_line)?;
        Ok((events, &lines[1..]))
    } else {
        Ok((Vec::new(), lines))
    }
}

fn tokenize_directive_tokens(inner: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in inner.chars() {
        if in_quote {
            current.push(ch);
            if ch == '"' {
                in_quote = false;
            }
        } else if ch == '"' {
            current.push(ch);
            in_quote = true;
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if in_quote {
        return Err("unclosed quote in directive line".to_string());
    }
    Ok(tokens)
}

fn parse_directive_line(line: &str) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    let inner = &line[1..line.len() - 1];
    let tokens = tokenize_directive_tokens(inner).map_err(|msg| {
        JianPuError::new(Span::new(0, line.len()), msg)
    })?;
    let mut events = Vec::new();

    for token in &tokens {
        let span = Span::new(0, token.len());

        let event = if let Some(rest) = token.strip_prefix("bpm=") {
            let bpm = rest.parse::<u32>().map_err(|_| {
                JianPuError::new(span.clone(), format!("invalid bpm value: {}", rest))
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
                    format!("label value must be a quoted string, got: {}", rest),
                ));
            }
            let text = rest[1..rest.len() - 1].to_string();
            ScoreEvent::LabelChange(text)
        } else {
            return Err(JianPuError::new(span, format!("unknown directive: '{}'", token)));
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
        'A' => NoteName::A, 'B' => NoteName::B, 'C' => NoteName::C,
        'D' => NoteName::D, 'E' => NoteName::E, 'F' => NoteName::F,
        'G' => NoteName::G,
        _ => return Err(JianPuError::new(span.clone(), format!("invalid note name: '{}'", name_char))),
    };

    let accidental = match chars.peek() {
        Some('b') => { chars.next(); Accidental::Flat }
        Some('#') => { chars.next(); Accidental::Sharp }
        _ => Accidental::Natural,
    };

    let octave_str: String = chars.collect();
    let octave = octave_str.parse::<u8>().map_err(|_| {
        JianPuError::new(span.clone(), format!("invalid octave in 'key={}': expected number", value))
    })?;

    Ok(ScoreEvent::KeyChange(KeyChange { note: Note { name, octave, accidental } }))
}

fn parse_time_value(value: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() != 2 {
        return Err(JianPuError::new(span.clone(), format!("invalid time signature: '{}'", value)));
    }
    let numerator = parts[0].parse::<u8>().map_err(|_| {
        JianPuError::new(span.clone(), format!("invalid time numerator: '{}'", parts[0]))
    })?;
    let denominator = parts[1].parse::<u8>().map_err(|_| {
        JianPuError::new(span.clone(), format!("invalid time denominator: '{}'", parts[1]))
    })?;
    if denominator == 0 {
        return Err(JianPuError::new(span, "time denominator cannot be zero".to_string()));
    }
    Ok(ScoreEvent::TimeSignatureChange { numerator, denominator })
}

fn beats_per_measure(num: u8, den: u8) -> u32 {
    (num as u32) * (16 / den as u32)
}

fn validate_beats(
    events: &[Spanned<ScoreEvent>],
    expected: u32,
    bar: usize,
) -> Result<(), JianPuError> {
    let mut total = 0u32;
    let mut note_idx = 0usize;

    for e in events {
        let beats = match &e.value {
            ScoreEvent::Note(n) => n.duration,
            ScoreEvent::Rest(r) => r.duration,
            ScoreEvent::Extension => 4,
            _ => 0,
        };
        if beats > 0 {
            note_idx += 1;
            total += beats;
            if total > expected {
                return Err(JianPuError::at_bar(bar, note_idx, format!(
                    "note exceeds measure boundary: measure has {} quarter-beats, cumulative is now {}",
                    expected, total
                )));
            }
        }
    }

    if total < expected {
        return Err(JianPuError::at_bar(bar, 0, format!(
            "incomplete measure: expected {} quarter-beats, got {}",
            expected, total
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parsed::PartColumn;

    fn notes_col(name: &str) -> PartColumn { PartColumn::Notes { name: name.to_string() } }
    fn lyrics_col(name: &str) -> PartColumn { PartColumn::Lyrics { name: name.to_string() } }

    #[test]
    fn single_unnamed_part_no_lyrics() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, None);
        assert!(result[0].lyrics.is_none());
        assert_eq!(result[0].score.events.len(), 7);
    }

    #[test]
    fn single_part_with_lyrics() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\ndo re mi fa\n";
        let parts = vec![notes_col(""), lyrics_col("")];
        let result = parse(content, &parts).unwrap();
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
        let result = parse(content, &parts).unwrap();
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
        let err = parse(content, &parts).unwrap_err();
        assert!(matches!(err.location, crate::error::Location::Bar { bar: 1, .. }));
    }

    #[test]
    fn allows_missing_trailing_lyrics_line_in_subsequent_bars() {
        // Bar 2 has no lyrics line — it should be padded with empty.
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n",
            "\n",
            "5 6 7 1\n",
        );
        let parts = vec![notes_col(""), lyrics_col("")];
        let result = parse(content, &parts).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].lyrics.as_ref().unwrap().syllables.len(), 4);
    }

    #[test]
    fn rejects_overfull_measure() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4 5\n";
        let parts = vec![notes_col("")];
        let err = parse(content, &parts).unwrap_err();
        assert!(matches!(err.location, crate::error::Location::Bar { bar: 1, note: 5 }));
    }

    #[test]
    fn rejects_underfull_measure() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3\n";
        let parts = vec![notes_col("")];
        let err = parse(content, &parts).unwrap_err();
        assert!(matches!(err.location, crate::error::Location::Bar { bar: 1, note: 0 }));
    }

    #[test]
    fn directive_row_is_optional() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
            "\n",
            "5 6 7 1\n",
        );
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
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
        let result = parse(content, &parts).unwrap();
        assert!(result[0].score.events.len() > 0);
    }

    #[test]
    fn rejects_unknown_directive() {
        let content = "(foo=bar)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        assert!(parse(content, &parts).is_err());
    }

    #[test]
    fn key_directive_parses_flat() {
        let content = "(time=4/4 key=Bb4 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        use crate::ast::parsed::{Accidental, ScoreEvent};
        let key_event = result[0].score.events.iter().find(|e| matches!(&e.value, ScoreEvent::KeyChange(_)));
        assert!(key_event.is_some());
        if let ScoreEvent::KeyChange(kc) = &key_event.unwrap().value {
            assert_eq!(kc.note.accidental, Accidental::Flat);
        }
    }

    #[test]
    fn label_directive_parsed() {
        let content = "(time=4/4 key=C4 bpm=120 label=\"Verse 1\")\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        use crate::ast::parsed::ScoreEvent;
        let label_event = result[0].score.events.iter()
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
        assert!(parse(content, &parts).is_err());
    }

    #[test]
    fn key_directive_parses_sharp() {
        let content = "(time=4/4 key=F#3 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        use crate::ast::parsed::{Accidental, ScoreEvent};
        let key_event = result[0].score.events.iter().find(|e| matches!(&e.value, ScoreEvent::KeyChange(_)));
        if let ScoreEvent::KeyChange(kc) = &key_event.unwrap().value {
            assert_eq!(kc.note.accidental, Accidental::Sharp);
        }
    }
}
