use crate::ast::parsed::{flatten_score_line_slots, PartDecl, ScoreLineRole};
use crate::error::{JianPuError, Span};

/// Resolves `"` ditto lines within each measure group.
///
/// A `"` on a data line means "same content as the closest preceding line of
/// the same score line role in this group." The directive line (starts with `(`)
/// is never a ditto source or target.
pub fn desugar_groups(
    groups: Vec<Vec<(String, usize)>>,
    declarations: &[PartDecl],
) -> Result<Vec<Vec<(String, usize)>>, JianPuError> {
    let slots = flatten_score_line_slots(declarations);
    groups
        .into_iter()
        .map(|group| {
            let padded = pad_implicit_ditto_group(&group, declarations, &slots)?;
            desugar_group(&padded, declarations, &slots)
        })
        .collect()
}

fn pad_implicit_ditto_group(
    group: &[(String, usize)],
    declarations: &[PartDecl],
    slots: &[crate::ast::parsed::ScoreLineSlot],
) -> Result<Vec<(String, usize)>, JianPuError> {
    let directive_count = if group
        .first()
        .map(|(l, _)| l.starts_with('('))
        .unwrap_or(false)
    {
        1
    } else {
        0
    };

    let directive_lines = group.get(..directive_count).unwrap_or(&[]);
    let data_lines = group.get(directive_count..).unwrap_or(&[]);

    let span = data_lines
        .last()
        .or(group.last())
        .map(|(_, off)| Span::new(*off, *off + 1))
        .unwrap_or(Span::new(0, 1));

    if data_lines.is_empty() {
        return Err(JianPuError::new(
            span,
            "expected at least one data line in measure group".to_string(),
        ));
    }

    if data_lines.len() > slots.len() {
        return Err(JianPuError::new(
            span,
            format!(
                "expected at most {} lines (one per score line), got {}",
                slots.len(),
                data_lines.len()
            ),
        ));
    }

    let pad_offset = data_lines.last().map(|(_, off)| *off).unwrap_or(0);
    let mut result_data: Vec<(String, usize)> = data_lines.to_vec();

    for i in data_lines.len()..slots.len() {
        let slot = slots.get(i).ok_or_else(|| {
            JianPuError::new(
                Span::new(0, 0),
                "internal invariant: score line slot missing for implicit ditto padding",
            )
        })?;
        let role = slot.role;
        let has_precedent =
            (0..result_data.len()).any(|j| slots.get(j).map(|s| s.role == role).unwrap_or(false));

        if has_precedent {
            result_data.push(("\"".to_string(), pad_offset));
        } else {
            let abbrev = track_abbreviation(declarations, slot.track_index);
            let hint = if role == ScoreLineRole::Lyrics {
                "write content, '\"' ditto, or '_' for no lyrics"
            } else {
                "write content or '\"' ditto"
            };
            return Err(JianPuError::new(
                Span::new(pad_offset, pad_offset + 1),
                format!("expected {} line for '{abbrev}'; {hint}", role_name(role)),
            ));
        }
    }

    let mut result = directive_lines.to_vec();
    result.extend(result_data);
    Ok(result)
}

fn desugar_group(
    group: &[(String, usize)],
    _declarations: &[PartDecl],
    slots: &[crate::ast::parsed::ScoreLineSlot],
) -> Result<Vec<(String, usize)>, JianPuError> {
    let directive_count = if group
        .first()
        .map(|(l, _)| l.starts_with('('))
        .unwrap_or(false)
    {
        1
    } else {
        0
    };

    let directive_lines = group.get(..directive_count).unwrap_or(&[]).to_vec();
    let data_lines = group.get(directive_count..).unwrap_or(&[]);

    let mut resolved: Vec<(String, usize)> = Vec::with_capacity(data_lines.len());

    for (i, (line, offset)) in data_lines.iter().enumerate() {
        if line == "\"" {
            if i >= slots.len() {
                resolved.push((line.clone(), *offset));
                continue;
            }
            let role = slots.get(i).map(|s| s.role).ok_or_else(|| {
                JianPuError::new(
                    Span::new(0, 0),
                    "internal invariant: score line slot missing for ditto line",
                )
            })?;
            let source = (0..resolved.len())
                .rev()
                .find(|&j| slots.get(j).map(|s| s.role == role).unwrap_or(false))
                .and_then(|j| resolved.get(j).map(|r| r.0.clone()));

            match source {
                Some(src_content) => resolved.push((src_content, *offset)),
                None => {
                    return Err(JianPuError::new(
                        Span::new(*offset, *offset + 1),
                        format!(
                            "ditto '\"' has no preceding {} line in this measure group",
                            role_name(role)
                        ),
                    ));
                }
            }
        } else {
            resolved.push((line.clone(), *offset));
        }
    }

    let mut result = directive_lines;
    result.extend(resolved);
    Ok(result)
}

fn role_name(role: ScoreLineRole) -> &'static str {
    match role {
        ScoreLineRole::Notes => "notes",
        ScoreLineRole::Lyrics => "lyrics",
        ScoreLineRole::Chord => "chord",
    }
}

fn track_abbreviation(declarations: &[PartDecl], track_index: usize) -> &str {
    declarations
        .get(track_index)
        .map(|d| d.abbreviation.as_str())
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parsed::PartKind;

    fn decl(name: &str, kind: PartKind) -> PartDecl {
        PartDecl {
            abbreviation: name.to_string(),
            display_name: name.to_string(),
            kind,
        }
    }

    fn group(lines: &[&str]) -> Vec<(String, usize)> {
        lines
            .iter()
            .enumerate()
            .map(|(i, l)| (l.to_string(), i * 10))
            .collect()
    }

    #[test]
    fn notes_ditto_copies_preceding_notes_line() {
        let groups = vec![group(&["1 2 3 4", "\""])];
        let declarations = vec![decl("A", PartKind::Notes), decl("B", PartKind::Notes)];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][1].0, "1 2 3 4");
    }

    #[test]
    fn lyrics_ditto_copies_preceding_lyrics_line() {
        let groups = vec![group(&["1 2 3 4", "hello world", "5 6 7 1", "\""])];
        let declarations = vec![
            decl("A", PartKind::NotesWithLyrics),
            decl("B", PartKind::NotesWithLyrics),
        ];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][3].0, "hello world");
    }

    #[test]
    fn chord_ditto_copies_preceding_chord_line() {
        let groups = vec![group(&["1 - - -", "1 2 3 4", "\"", "5 6 7 1"])];
        let declarations = vec![
            decl("main", PartKind::Chord),
            decl("A", PartKind::Notes),
            decl("main2", PartKind::Chord),
            decl("B", PartKind::Notes),
        ];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][2].0, "1 - - -");
    }

    #[test]
    fn notes_ditto_does_not_copy_lyrics_line() {
        let groups = vec![group(&["1 2 3 4", "hello world", "\""])];
        let declarations = vec![
            decl("A", PartKind::NotesWithLyrics),
            decl("B", PartKind::Notes),
        ];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][2].0, "1 2 3 4");
    }

    #[test]
    fn chained_ditto_resolves_transitively() {
        let groups = vec![group(&["1 2 3 4", "\"", "\""])];
        let declarations = vec![
            decl("A", PartKind::Notes),
            decl("B", PartKind::Notes),
            decl("C", PartKind::Notes),
        ];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][1].0, "1 2 3 4");
        assert_eq!(result[0][2].0, "1 2 3 4");
    }

    #[test]
    fn ditto_with_no_preceding_line_is_an_error() {
        let groups = vec![group(&["\""])];
        let declarations = vec![decl("A", PartKind::Notes)];
        let err = desugar_groups(groups, &declarations).unwrap_err();
        assert!(
            err.message.contains("no preceding notes line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn ditto_with_no_preceding_line_of_same_type_is_an_error() {
        let groups = vec![group(&["1 2 3 4", "\""])];
        let declarations = vec![decl("A", PartKind::NotesWithLyrics)];
        let err = desugar_groups(groups, &declarations).unwrap_err();
        assert!(
            err.message.contains("no preceding lyrics line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn directive_line_is_not_a_ditto_target() {
        let groups = vec![group(&["(time=4/4)", "\""])];
        let declarations = vec![decl("A", PartKind::Notes)];
        let err = desugar_groups(groups, &declarations).unwrap_err();
        assert!(
            err.message.contains("no preceding notes line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn directive_line_is_not_a_ditto_source() {
        let groups = vec![group(&["(time=4/4)", "1 2 3 4", "\""])];
        let declarations = vec![decl("A", PartKind::Notes), decl("B", PartKind::Notes)];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][0].0, "(time=4/4)");
        assert_eq!(result[0][2].0, "1 2 3 4");
    }

    #[test]
    fn non_ditto_lines_are_passed_through_unchanged() {
        let groups = vec![group(&["1 2 3 4", "hello"])];
        let declarations = vec![decl("A", PartKind::NotesWithLyrics)];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][0].0, "1 2 3 4");
        assert_eq!(result[0][1].0, "hello");
    }

    #[test]
    fn multiple_groups_are_desugared_independently() {
        let groups = vec![group(&["1 2 3 4"]), group(&["\""])];
        let declarations = vec![decl("A", PartKind::Notes)];
        let err = desugar_groups(groups, &declarations).unwrap_err();
        assert!(
            err.message.contains("no preceding notes line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn omitted_trailing_notes_line_is_padded_as_implicit_ditto() {
        let groups = vec![group(&["1 2 3 4"])];
        let declarations = vec![decl("A", PartKind::Notes), decl("B", PartKind::Notes)];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][0].0, "1 2 3 4");
        assert_eq!(result[0][1].0, "1 2 3 4");
    }

    #[test]
    fn omitted_trailing_lines_pad_as_ditto_when_precedent_exists() {
        let groups = vec![group(&["1 - - -", "1 2 3 4", "hello"])];
        let declarations = vec![
            decl("main", PartKind::Chord),
            decl("A", PartKind::NotesWithLyrics),
            decl("B", PartKind::NotesWithLyrics),
        ];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][3].0, "1 2 3 4");
        assert_eq!(result[0][4].0, "hello");
    }

    #[test]
    fn omitted_trailing_lyrics_without_precedent_is_an_error() {
        let groups = vec![group(&["1 2 3 4"])];
        let declarations = vec![decl("A", PartKind::NotesWithLyrics)];
        let err = desugar_groups(groups, &declarations).unwrap_err();
        assert!(
            err.message.contains("expected lyrics line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn ditto_can_copy_underscore_no_lyrics_marker() {
        let groups = vec![group(&["1 2 3 4", "_", "\""])];
        let declarations = vec![
            decl("A", PartKind::NotesWithLyrics),
            decl("B", PartKind::NotesWithLyrics),
        ];
        let result = desugar_groups(groups, &declarations).unwrap();
        assert_eq!(result[0][3].0, "_");
    }
}
