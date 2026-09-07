mod octave_shift;
pub use octave_shift::{shift_part_octave, shift_range_octave, ByteRange, ShiftRangeOctaveResult};

use crate::parser::parts_parser::SourcePartMode;

pub enum PartMode {
    Chords,
    Notes,
    Percussion,
    Follow { target: String },
}

impl PartMode {
    /// Builds a `PartMode` from the same [`SourcePartMode`] tags the
    /// parser produces (and the wasm boundary's `part-declaration-mode`
    /// enum mirrors) instead of hand-parsing a `"chords"`/"follow[...]"`
    /// wire string — see item 3 of `TODO-cross-boundary-invariants.md`.
    /// `follow_target` is only consulted for `SourcePartMode::Follow`.
    pub fn from_source_mode(kind: SourcePartMode, follow_target: Option<String>) -> Self {
        match kind {
            SourcePartMode::Chords => Self::Chords,
            SourcePartMode::Notes => Self::Notes,
            SourcePartMode::Percussion => Self::Percussion,
            SourcePartMode::Follow => Self::Follow {
                target: follow_target.unwrap_or_default(),
            },
        }
    }

    pub fn to_rhs_str(&self) -> String {
        match self {
            Self::Chords => "chords".to_owned(),
            Self::Notes => "notes".to_owned(),
            Self::Percussion => "percussion".to_owned(),
            Self::Follow { target } => format!("follow[{target}]"),
        }
    }
}

pub fn update_part_declaration(
    source: &str,
    abbreviation: &str,
    new_mode: &PartMode,
    new_soundfont: Option<&str>,
    new_volume: Option<u8>,
    new_octave_offset: Option<i8>,
) -> Option<String> {
    let lines: Vec<&str> = source.split('\n').collect();

    let parts_index = lines.iter().position(|line| line.trim() == "# parts")?;

    let target_index = lines
        .iter()
        .enumerate()
        .skip(parts_index + 1)
        .find(|(_, line)| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }
            if trimmed.starts_with("# ") {
                return false;
            }
            let Some(eq_pos) = line.find('=') else {
                return false;
            };
            let lhs = line[..eq_pos].trim();
            let line_abbr = if let Some(bracket_start) = lhs.rfind('[') {
                lhs[bracket_start + 1..].trim_end_matches(']')
            } else {
                lhs
            };
            line_abbr == abbreviation
        })
        .map(|(index, _)| index)?;

    let line = lines.get(target_index)?;
    let eq_pos = line.find('=')?;
    let lhs_with_eq = &line[..eq_pos + 1];

    let soundfont_suffix = new_soundfont
        .map(|sf| format!(" \"{sf}\""))
        .unwrap_or_default();

    let volume_suffix = match new_volume {
        Some(v) if v != 100 => format!(" {v}%"),
        _ => String::new(),
    };

    let octave_suffix = match new_octave_offset {
        Some(offset) if offset != 0 => {
            if offset > 0 {
                format!(" +{offset}")
            } else {
                format!(" {offset}")
            }
        }
        _ => String::new(),
    };

    let new_rhs = new_mode.to_rhs_str();
    let new_line =
        format!("{lhs_with_eq} {new_rhs}{soundfont_suffix}{volume_suffix}{octave_suffix}");

    let result = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == target_index {
                new_line.as_str()
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Some(result)
}

#[cfg(test)]
mod tests;
