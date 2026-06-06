use crate::ast::parsed::PartColumn;
use crate::error::{JianPuError, Span};

#[allow(dead_code)]
/// Resolves `"` ditto lines within each measure group.
///
/// A `"` on a data line means "same content as the closest preceding line of
/// the same column type in this group." The directive line (starts with `(`)
/// is never a ditto source or target.
///
/// `parts` maps each data-line position to its column type.
pub fn desugar_groups(
    groups: Vec<Vec<(String, usize)>>,
    parts: &[PartColumn],
) -> Result<Vec<Vec<(String, usize)>>, JianPuError> {
    groups
        .into_iter()
        .map(|group| desugar_group(group, parts))
        .collect()
}

#[allow(dead_code)]
fn desugar_group(
    group: Vec<(String, usize)>,
    parts: &[PartColumn],
) -> Result<Vec<(String, usize)>, JianPuError> {
    // Directive line (starts with `(`) is never a ditto target — pass it through.
    let directive_count = if group
        .first()
        .map(|(l, _)| l.starts_with('('))
        .unwrap_or(false)
    {
        1
    } else {
        0
    };

    let directive_lines = group[..directive_count].to_vec();
    let data_lines = &group[directive_count..];

    let mut resolved: Vec<(String, usize)> = Vec::with_capacity(data_lines.len());

    for (i, (line, offset)) in data_lines.iter().enumerate() {
        if line == "\"" {
            // Guard: if i >= parts.len() the interleaved parser will emit a better error.
            if i >= parts.len() {
                resolved.push((line.clone(), *offset));
                continue;
            }
            let col_type = column_type(&parts[i]);
            let source = (0..resolved.len())
                .rev()
                .find(|&j| j < parts.len() && column_type(&parts[j]) == col_type)
                .map(|j| resolved[j].0.clone());

            match source {
                Some(src_content) => {
                    // Keep the ditto's own byte offset so error spans point here.
                    resolved.push((src_content, *offset));
                }
                None => {
                    return Err(JianPuError::new(
                        Span::new(*offset, *offset + 1),
                        format!(
                            "ditto '\"' has no preceding {} line in this measure group",
                            col_type_name(&parts[i])
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

#[derive(PartialEq)]
#[allow(dead_code)]
enum ColType {
    Notes,
    Lyrics,
    Chord,
}

#[allow(dead_code)]
fn column_type(col: &PartColumn) -> ColType {
    match col {
        PartColumn::Notes { .. } => ColType::Notes,
        PartColumn::Lyrics { .. } => ColType::Lyrics,
        PartColumn::Chord { .. } => ColType::Chord,
    }
}

#[allow(dead_code)]
fn col_type_name(col: &PartColumn) -> &'static str {
    match col {
        PartColumn::Notes { .. } => "notes",
        PartColumn::Lyrics { .. } => "lyrics",
        PartColumn::Chord { .. } => "chord",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(name: &str) -> PartColumn {
        PartColumn::Notes {
            name: name.to_string(),
        }
    }
    fn lyrics(name: &str) -> PartColumn {
        PartColumn::Lyrics {
            name: name.to_string(),
        }
    }
    fn chord(name: &str) -> PartColumn {
        PartColumn::Chord {
            name: name.to_string(),
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
        let parts = vec![notes("A"), notes("B")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][1].0, "1 2 3 4");
    }
}
