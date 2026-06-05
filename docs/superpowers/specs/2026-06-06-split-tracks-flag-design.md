# Design: `--split-tracks` flag and `--output` stem change

**Date:** 2026-06-06

## Summary

Add a `--split-tracks` flag to `jianpu generate pdf/svg/midi` that generates one output file per track instead of a single combined file. Alongside this, change `--output` to accept a stem (no extension) across all formats.

## CLI Changes

Each `GenerateFormat` variant (`Pdf`, `Svg`, `Midi`) gets two new/changed fields:

- `output: Option<PathBuf>` — now treated as a **stem** (no extension). The format extension is always appended at write time via `.with_extension(ext)`. This is a breaking change from the current behavior where `--output` accepted a full path including extension.
- `split_tracks: bool` — new `#[arg(long)]` field added to each variant.

Usage examples:
```
jianpu generate pdf song.jianpu --split-tracks
# → song - A1&T.pdf, song - A2.pdf, song - S1.pdf, song - S2.pdf

jianpu generate pdf song.jianpu --output out/song --split-tracks
# → out/song - A1&T.pdf, out/song - A2.pdf, ...

jianpu generate pdf song.jianpu --output out/song
# → out/song.pdf
```

## Implementation

### `default_output` update

The function already builds `<stem> - <tracks>` names. With the new design, `--output` (if provided) is used as-is as the stem. The extension is always appended at write time. No structural change needed to `default_output`.

### New helper: `collect_track_names`

```rust
fn collect_track_names(score: &ast::grouped::Score) -> Vec<String>
```

Returns all unique track names present in the score (in order of first appearance). Used when `--split-tracks` is true.

### `run_generate` logic when `split_tracks = true`

1. Parse and group the score once.
2. Determine the candidate track names: if `--tracks` was given, use those; otherwise call `collect_track_names`.
3. For each track name:
   - Clone the score.
   - Call `filter_tracks(&mut score_clone, &[track_name])`.
   - Derive output path: `<stem> - <track_name>.<ext>`.
   - Generate and write the file.

### Edge cases

- **`--split-tracks` with `--tracks`:** Only split the tracks listed in `--tracks`, not all tracks. This lets users generate a subset of tracks as separate files.
- **Score with no named tracks + `--split-tracks`:** Print a warning to stderr and fall back to generating a single file (no track suffix).
- **`--output` without `--split-tracks`:** Stem is used as the base path, extension is appended. Single file output.

## Scope

Changes are confined to `src/main.rs`. No changes needed to parser, renderer, layout, pdf, midi, or svg modules.
