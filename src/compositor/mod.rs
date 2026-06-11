pub mod types;
pub use types::*;

use crate::compiler::types::{Decoration, ElementContent, MeasureBlock, MeasureRow};
use crate::layout::new_types::{Page, System};
use crate::render_config::RenderConfig;

const PAGE_MARGIN: f32 = 25.0;

#[allow(dead_code)]
fn is_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

/// Infer the row height (in row-units) for a single MeasureRow based on its elements.
fn infer_row_height(row: &MeasureRow) -> u32 {
    if row
        .elements
        .iter()
        .any(|e| matches!(e.content, ElementContent::Lyric(_)))
    {
        return 4;
    }
    let has_note_head = row
        .elements
        .iter()
        .any(|e| matches!(e.content, ElementContent::NoteHead { .. }));
    let has_rest = row
        .elements
        .iter()
        .any(|e| matches!(e.content, ElementContent::Rest { .. }));
    if !has_note_head && !has_rest {
        return 2;
    }
    3
}

/// Compute the column width of a MeasureBlock (= BarLine column + 1).
fn block_column_width(block: &MeasureBlock) -> u32 {
    block
        .rows
        .first()
        .and_then(|row| {
            row.elements
                .iter()
                .find(|e| e.content == ElementContent::BarLine)
        })
        .map(|e| e.column + 1)
        .unwrap_or(1)
}

/// Whether the first block has any directive decoration (Bpm or TimeSignature).
fn has_directive(block: &MeasureBlock) -> bool {
    block
        .decorations
        .iter()
        .any(|d| matches!(d, Decoration::Bpm(_) | Decoration::TimeSignature { .. }))
}

/// Total row-units for a system (sum over all parts in the given block).
fn system_row_height(block: &MeasureBlock) -> u32 {
    block.rows.iter().map(infer_row_height).sum()
}

// ── Geometry context passed through system composition ───────────────────────

struct SysCtx {
    system_y: f32,
    directive_extra_y: f32,
    system_note_rows: u32,
    column_width: f32,
    label_width_pt: f32,
    row_height_pt: f32,
    note_number_width: f32,
    base_font_size: f32,
}

impl SysCtx {
    fn x_col(&self, measure_start: u32, col: u32) -> f32 {
        PAGE_MARGIN + self.label_width_pt + (measure_start + col) as f32 * self.column_width
    }
}

// ── Per-measure helper functions ─────────────────────────────────────────────

fn emit_decorations(
    ctx: &SysCtx,
    block: &MeasureBlock,
    measure_start: u32,
    out: &mut Vec<AbsoluteElement>,
) {
    let col_w = block_column_width(block);
    let block_x_base = PAGE_MARGIN + ctx.label_width_pt + measure_start as f32 * ctx.column_width;
    let slot_width = col_w as f32 * ctx.column_width;
    let directive_y = ctx.system_y + ctx.row_height_pt * 0.5;

    for decoration in &block.decorations {
        match decoration {
            Decoration::Bpm(bpm) => {
                out.push(AbsoluteElement {
                    x: block_x_base + slot_width * 0.5,
                    y: directive_y,
                    content: AbsoluteContent::Text {
                        content: format!("♩={bpm}"),
                        font_size: ctx.base_font_size,
                        anchor: TextAnchor::Middle,
                        baseline: DominantBaseline::Middle,
                        font: FontFamily::SansSerif,
                        weight: FontWeight::Normal,
                        italic: false,
                    },
                });
            }
            Decoration::TimeSignature {
                numerator,
                denominator,
            } => {
                let x = block_x_base + slot_width * 0.5;
                out.push(AbsoluteElement {
                    x,
                    y: directive_y - ctx.row_height_pt * 0.25,
                    content: AbsoluteContent::Text {
                        content: numerator.to_string(),
                        font_size: ctx.base_font_size,
                        anchor: TextAnchor::Middle,
                        baseline: DominantBaseline::Middle,
                        font: FontFamily::SansSerif,
                        weight: FontWeight::Normal,
                        italic: false,
                    },
                });
                let line_width = slot_width * 0.6;
                out.push(AbsoluteElement {
                    x: x - line_width * 0.5,
                    y: directive_y,
                    content: AbsoluteContent::Underline {
                        width: line_width,
                        level: 0,
                    },
                });
                out.push(AbsoluteElement {
                    x,
                    y: directive_y + ctx.row_height_pt * 0.25,
                    content: AbsoluteContent::Text {
                        content: denominator.to_string(),
                        font_size: ctx.base_font_size,
                        anchor: TextAnchor::Middle,
                        baseline: DominantBaseline::Middle,
                        font: FontFamily::SansSerif,
                        weight: FontWeight::Normal,
                        italic: false,
                    },
                });
            }
            Decoration::SectionLabel(label) => {
                out.push(AbsoluteElement {
                    x: block_x_base,
                    y: directive_y,
                    content: AbsoluteContent::Text {
                        content: label.clone(),
                        font_size: ctx.base_font_size,
                        anchor: TextAnchor::Start,
                        baseline: DominantBaseline::Middle,
                        font: FontFamily::SansSerif,
                        weight: FontWeight::Bold,
                        italic: true,
                    },
                });
            }
            Decoration::BarNumber(n) => {
                out.push(AbsoluteElement {
                    x: block_x_base,
                    y: directive_y,
                    content: AbsoluteContent::Text {
                        content: n.to_string(),
                        font_size: ctx.base_font_size * 0.7,
                        anchor: TextAnchor::Start,
                        baseline: DominantBaseline::Ideographic,
                        font: FontFamily::SansSerif,
                        weight: FontWeight::Normal,
                        italic: false,
                    },
                });
            }
        }
    }
}

fn emit_underline_element(
    ctx: &SysCtx,
    measure_start: u32,
    from_column: u32,
    last_head_column: u32,
    level: u32,
    part_y_base: f32,
    out: &mut Vec<AbsoluteElement>,
) {
    let rh = ctx.row_height_pt;
    let x1 = ctx.x_col(measure_start, from_column) + ctx.column_width * 0.1;
    let x2 = ctx.x_col(measure_start, last_head_column)
        + ctx.column_width * 0.5
        + ctx.note_number_width * 0.5;
    let y = part_y_base + rh * 2.0 + rh * 0.1 + level as f32 * (rh * 0.15);
    out.push(AbsoluteElement {
        x: x1,
        y,
        content: AbsoluteContent::Underline {
            width: (x2 - x1).max(0.0),
            level,
        },
    });
}

fn emit_note_dash_element(
    ctx: &SysCtx,
    x_base: f32,
    part_y_base: f32,
    out: &mut Vec<AbsoluteElement>,
) {
    let rh = ctx.row_height_pt;
    out.push(AbsoluteElement {
        x: x_base + ctx.column_width * 0.5,
        y: part_y_base + rh * 1.5,
        content: AbsoluteContent::Text {
            content: "—".to_string(),
            font_size: ctx.base_font_size,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::Monospace,
            weight: FontWeight::Normal,
            italic: false,
        },
    });
}

fn emit_row_content(
    ctx: &SysCtx,
    row: &MeasureRow,
    measure_start: u32,
    part_y_base: f32,
    part_idx: usize,
    out: &mut Vec<AbsoluteElement>,
) {
    let rh = ctx.row_height_pt;
    for element in &row.elements {
        let x_base = ctx.x_col(measure_start, element.column);
        match &element.content {
            ElementContent::NoteHead {
                pitch,
                octave,
                dotted,
            } => {
                out.push(AbsoluteElement {
                    x: x_base + ctx.column_width * 0.5,
                    y: part_y_base + rh * 1.5,
                    content: AbsoluteContent::NoteHead {
                        pitch: pitch.clone(),
                        octave: *octave,
                        dotted: *dotted,
                    },
                });
            }
            ElementContent::Rest { dotted } => {
                out.push(AbsoluteElement {
                    x: x_base + ctx.column_width * 0.5,
                    y: part_y_base + rh * 1.5,
                    content: AbsoluteContent::Rest { dotted: *dotted },
                });
            }
            ElementContent::ChordSymbol(s) => {
                out.push(AbsoluteElement {
                    x: x_base,
                    y: part_y_base + rh * 1.5,
                    content: AbsoluteContent::ChordSymbol(s.clone()),
                });
            }
            ElementContent::Underline {
                from_column,
                last_head_column,
                level,
                ..
            } => emit_underline_element(
                ctx,
                measure_start,
                *from_column,
                *last_head_column,
                *level,
                part_y_base,
                out,
            ),
            ElementContent::TieOrSlur {
                from_column,
                to_column,
            } => {
                let x1 = ctx.x_col(measure_start, *from_column) + ctx.column_width * 0.5;
                let x2 = ctx.x_col(measure_start, *to_column) + ctx.column_width * 0.5;
                out.push(AbsoluteElement {
                    x: x1,
                    y: part_y_base + rh,
                    content: AbsoluteContent::TieOrSlur { width: x2 - x1 },
                });
            }
            ElementContent::TieOrSlurClose { to_column } => {
                let x2 = ctx.x_col(measure_start, *to_column) + ctx.column_width * 0.5;
                let x1 = ctx.x_col(measure_start, *to_column);
                if x2 > x1 {
                    out.push(AbsoluteElement {
                        x: x1,
                        y: part_y_base + rh,
                        content: AbsoluteContent::TieOrSlur { width: x2 - x1 },
                    });
                }
            }
            ElementContent::BarLine => {
                // Only emit from first part to avoid duplicates
                if part_idx == 0 {
                    let bh = ctx.system_note_rows as f32 * rh;
                    out.push(AbsoluteElement {
                        x: x_base + ctx.column_width * 0.5,
                        y: ctx.system_y + ctx.directive_extra_y + rh,
                        content: AbsoluteContent::BarLine { height: bh },
                    });
                }
            }
            ElementContent::Lyric(text) => {
                out.push(AbsoluteElement {
                    x: x_base + ctx.column_width * 0.5,
                    y: part_y_base + rh * 3.0,
                    content: AbsoluteContent::Lyric(text.clone()),
                });
            }
            ElementContent::NoteDash => {
                emit_note_dash_element(ctx, x_base, part_y_base, out);
            }
        }
    }
}

fn emit_system(
    system: &System,
    system_y: f32,
    config: &RenderConfig,
    page_width_pt: f32,
    out: &mut Vec<AbsoluteElement>,
) {
    let Some(first_block) = system.measures.first() else {
        return;
    };
    let row_height_pt = config.row_height as f32;
    let label_width_pt = config.label_width as f32;

    let system_has_directive = has_directive(first_block);
    let directive_extra_y = if system_has_directive {
        row_height_pt
    } else {
        0.0
    };
    let system_note_rows: u32 = system_row_height(first_block);

    let usable_width = page_width_pt - 2.0 * PAGE_MARGIN - label_width_pt;
    let total_columns: u32 = system.measures.iter().map(block_column_width).sum();
    let column_width = if total_columns > 0 {
        usable_width / total_columns as f32
    } else {
        usable_width
    };

    let ctx = SysCtx {
        system_y,
        directive_extra_y,
        system_note_rows,
        column_width,
        label_width_pt,
        row_height_pt,
        note_number_width: config.note_number_width as f32,
        base_font_size: row_height_pt * 0.6,
    };

    // Opening bar line
    out.push(AbsoluteElement {
        x: PAGE_MARGIN + label_width_pt,
        y: system_y + directive_extra_y + row_height_pt,
        content: AbsoluteContent::BarLine {
            height: system_note_rows as f32 * row_height_pt,
        },
    });

    // Row labels
    for label in &system.row_labels {
        if !label.text.is_empty() {
            out.push(AbsoluteElement {
                x: PAGE_MARGIN + label_width_pt * 0.5,
                y: system_y + directive_extra_y + label.y_offset_pt + row_height_pt * 1.5,
                content: AbsoluteContent::Text {
                    content: label.text.clone(),
                    font_size: ctx.base_font_size * 0.8,
                    anchor: TextAnchor::Middle,
                    baseline: DominantBaseline::Middle,
                    font: FontFamily::SansSerif,
                    weight: FontWeight::Normal,
                    italic: false,
                },
            });
        }
    }

    // Decorations for each measure
    let mut measure_start_col: u32 = 0;
    for block in &system.measures {
        emit_decorations(&ctx, block, measure_start_col, out);
        measure_start_col += block_column_width(block);
    }

    // Per-part, per-measure content
    let mut part_y_base = system_y + directive_extra_y;
    for (part_idx, part_row) in first_block.rows.iter().enumerate() {
        let part_row_units = infer_row_height(part_row);
        let mut measure_start_col: u32 = 0;
        for block in &system.measures {
            let col_w = block_column_width(block);
            if let Some(row) = block.rows.get(part_idx) {
                emit_row_content(&ctx, row, measure_start_col, part_y_base, part_idx, out);
            }
            measure_start_col += col_w;
        }
        part_y_base += part_row_units as f32 * row_height_pt;
    }
}

// ── Header / footer helpers ───────────────────────────────────────────────────

fn emit_header(
    page: &Page,
    row_height_pt: f32,
    base_font_size: f32,
    out: &mut Vec<AbsoluteElement>,
) {
    let page_width_pt = page.page_width_pt;
    let title_y = PAGE_MARGIN + row_height_pt * 0.75;
    out.push(AbsoluteElement {
        x: page_width_pt / 2.0,
        y: title_y,
        content: AbsoluteContent::Text {
            content: page.header.title.clone(),
            font_size: row_height_pt * 1.5,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        },
    });

    let subtitle_author_y = PAGE_MARGIN + row_height_pt * 2.25;
    if let Some(subtitle) = &page.header.subtitle {
        out.push(AbsoluteElement {
            x: page_width_pt / 2.0,
            y: subtitle_author_y,
            content: AbsoluteContent::Text {
                content: subtitle.clone(),
                font_size: row_height_pt * 0.8,
                anchor: TextAnchor::Middle,
                baseline: DominantBaseline::Middle,
                font: FontFamily::SansSerif,
                weight: FontWeight::Normal,
                italic: true,
            },
        });
    }
    out.push(AbsoluteElement {
        x: page_width_pt - PAGE_MARGIN,
        y: subtitle_author_y,
        content: AbsoluteContent::Text {
            content: page.header.author.clone(),
            font_size: base_font_size,
            anchor: TextAnchor::End,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        },
    });
}

fn emit_footer(
    page: &Page,
    row_height_pt: f32,
    base_font_size: f32,
    out: &mut Vec<AbsoluteElement>,
) {
    let footer_y = page.page_height_pt - PAGE_MARGIN - row_height_pt * 0.5;
    out.push(AbsoluteElement {
        x: page.page_width_pt / 2.0,
        y: footer_y,
        content: AbsoluteContent::Text {
            content: format!("{}/{}", page.footer.page, page.footer.total),
            font_size: base_font_size,
            anchor: TextAnchor::Middle,
            baseline: DominantBaseline::Middle,
            font: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            italic: false,
        },
    });
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn compose(pages: &[Page], config: &RenderConfig) -> Vec<AbsolutePage> {
    let row_height_pt = config.row_height as f32;
    let base_font_size = row_height_pt * 0.6;

    pages
        .iter()
        .map(|page| {
            let has_subtitle = page.header.subtitle.is_some();
            let header_rows: u32 = if has_subtitle { 3 } else { 2 };

            let mut elements: Vec<AbsoluteElement> = Vec::new();
            emit_header(page, row_height_pt, base_font_size, &mut elements);
            emit_footer(page, row_height_pt, base_font_size, &mut elements);

            let non_empty_systems: Vec<_> = page
                .systems
                .iter()
                .filter(|s| !s.measures.is_empty())
                .collect();
            let mut system_y = PAGE_MARGIN + header_rows as f32 * row_height_pt;
            for (i, system) in non_empty_systems.iter().enumerate() {
                let first_block = system.measures.first();
                let sys_note_rows = first_block.map(system_row_height).unwrap_or(0);
                let directive_extra = first_block
                    .map(|b| u32::from(has_directive(b)))
                    .unwrap_or(0);
                let system_total_rows = sys_note_rows + directive_extra;

                emit_system(system, system_y, config, page.page_width_pt, &mut elements);
                system_y += system_total_rows as f32 * row_height_pt;

                if i + 1 < non_empty_systems.len() {
                    elements.push(AbsoluteElement {
                        x: PAGE_MARGIN,
                        y: system_y,
                        content: AbsoluteContent::HorizontalLine {
                            width: page.page_width_pt - 2.0 * PAGE_MARGIN,
                        },
                    });
                }
            }

            AbsolutePage {
                width_pt: page.page_width_pt,
                height_pt: page.page_height_pt,
                elements,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
