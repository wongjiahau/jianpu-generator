# Split Tracks Flag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--split-tracks` flag to all `jianpu generate` subcommands that generates one output file per track, and change `--output` to accept a stem (no extension) across all formats.

**Architecture:** All changes are confined to `src/main.rs` and `src/ast/grouped.rs`. The `Score` type needs `Clone` so split-tracks can filter a per-track copy. The `default_output` function is replaced by `output_stem` (returns a stem without extension); all call sites append the extension explicitly. A new `collect_track_names` helper gathers unique track names from the score. The split-tracks loop clones the score, filters to one track, generates, and writes.

**Tech Stack:** Rust, Clap 4 (derive), existing pdf/svg/midi write functions.

---

### Task 1: Make Score cloneable

**Files:**
- Modify: `src/ast/grouped.rs`

- [ ] **Step 1: Add `#[derive(Clone)]` to the public types that need it**

In `src/ast/grouped.rs`, add `#[derive(Clone)]` to `Metadata`, `Notes`, `Lyrics`, `PartSlice`, `MultiPartMeasure`, and `Score`. (`TimeSignature`, `NoteEvent`, `GroupedNote`, `GroupedRest` already have it. `JianPuPitch`, `KeyChange`, `Syllable` in `parsed.rs` already have it.)

The file should look like:

```rust
use crate::ast::parsed::{JianPuPitch, KeyChange, Syllable};

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    pub row_height: u32,
    pub max_columns: u32,
    pub label_width: u32,
    pub note_number_width: u32,
}

#[derive(Clone)]
pub struct Notes {
    pub events: Vec<NoteEvent>,
}

#[derive(Clone)]
pub struct Lyrics {
    pub syllables: Vec<Syllable>,
}

#[derive(Clone)]
pub struct PartSlice {
    pub name: Option<String>,
    pub notes: Notes,
    pub lyrics: Option<Lyrics>,
}

#[derive(Clone)]
pub struct MultiPartMeasure {
    pub time_signature: Option<TimeSignature>,
    pub bpm: Option<u32>,
    pub key: Option<KeyChange>,
    pub label: Option<String>,
    pub parts: Vec<PartSlice>,
}

#[derive(Clone)]
pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<MultiPartMeasure>,
}
```

(Keep the `pub(crate)` intermediate types unchanged — they don't need Clone.)

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src/ast/grouped.rs
git commit -m "feat: derive Clone for Score and related types"
```

---

### Task 2: Implement stem-based `--output` (TDD)

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/integration.rs`

- [ ] **Step 1: Write a failing integration test for stem output**

In `tests/integration.rs`, add this test after the existing `generate_pdf_produces_pdf` test:

```rust
#[test]
fn output_stem_appends_extension() {
    let input_path = "/tmp/test_stem.jianpu";
    let output_stem = "/tmp/test_stem_out";
    let expected_output = "/tmp/test_stem_out.pdf";
    fs::write(input_path, basic_jianpu_input()).unwrap();
    let _ = fs::remove_file(expected_output);

    let status = jianpu_cmd()
        .args(["generate", "pdf", input_path, "--output", output_stem])
        .status()
        .unwrap();

    assert!(status.success(), "generate pdf command failed");
    let bytes = fs::read(expected_output).expect("output file not found at expected stem path");
    assert!(bytes.starts_with(b"%PDF"), "output is not a valid PDF");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(expected_output);
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test --test integration output_stem_appends_extension 2>&1
```

Expected: FAIL — the current binary writes to the full path argument, so `test_stem_out.pdf` is not found (the binary wrote to `test_stem_out` with no extension).

- [ ] **Step 3: Replace `default_output` with `output_stem` in `src/main.rs`**

Delete the existing `default_output` function and replace it with:

```rust
fn output_stem(input: &Path, tracks: &[String], output: Option<&Path>) -> PathBuf {
    match output {
        Some(out) => out.to_path_buf(),
        None => {
            let stem = input.file_stem().unwrap_or_default().to_string_lossy();
            let suffix = if tracks.is_empty() {
                stem.into_owned()
            } else {
                format!("{} - {}", stem, tracks.join("&"))
            };
            input.with_file_name(suffix)
        }
    }
}
```

- [ ] **Step 4: Update `run_generate` to use `output_stem` and append extension at write time**

In each `GenerateFormat` arm, change the output path computation. The pattern for every arm is the same: compute the stem, then append the extension.

**Pdf arm** — replace:
```rust
let output_path = output.unwrap_or_else(|| default_output(&input, &tracks, "pdf"));
```
with:
```rust
let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("pdf");
```

**Svg arm** — replace:
```rust
let output_path = output.unwrap_or_else(|| default_output(&input, &tracks, "svg"));
```
with:
```rust
let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("svg");
```

**Midi arm** — replace:
```rust
let output_path = output.unwrap_or_else(|| default_output(&input, &tracks, "mid"));
```
with:
```rust
let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("mid");
```

- [ ] **Step 5: Update the existing integration tests to pass stems instead of full paths**

In `tests/integration.rs`, in `generate_pdf_produces_pdf`, change:
```rust
let output_path = "/tmp/test_score.pdf";
// ...
.args(["generate", "pdf", input_path, output_path])
```
to:
```rust
let output_stem_arg = "/tmp/test_score";
let output_path = "/tmp/test_score.pdf";
// ...
.args(["generate", "pdf", input_path, "--output", output_stem_arg])
```

In `generate_midi_produces_midi`, change:
```rust
let output_path = "/tmp/test_score.mid";
// ...
.args(["generate", "midi", input_path, output_path])
```
to:
```rust
let output_stem_arg = "/tmp/test_score_midi_out";
let output_path = "/tmp/test_score_midi_out.mid";
// ...
.args(["generate", "midi", input_path, "--output", output_stem_arg])
```

(Using distinct stem names avoids any timing collision between parallel tests.)

- [ ] **Step 6: Run all tests to confirm they pass**

```bash
cargo test 2>&1
```

Expected: all tests pass including `output_stem_appends_extension`.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs tests/integration.rs
git commit -m "feat: change --output to accept a stem; extension is always appended"
```

---

### Task 3: Implement `--split-tracks` (TDD)

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/integration.rs`

- [ ] **Step 1: Write a failing integration test for `--split-tracks`**

Add a helper and two tests to `tests/integration.rs`:

```rust
fn multi_track_jianpu_input() -> &'static str {
    concat!(
        "[metadata]\n",
        "title = \"test score\"\n",
        "author = \"tester\"\n",
        "parts = notes:S1 lyrics:S1 notes:S2 lyrics:S2\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "do re mi fa\n",
        "5 6 7 1\n",
        "sol la ti do\n",
    )
}

#[test]
fn split_tracks_generates_one_pdf_per_track() {
    let input_path = "/tmp/test_split.jianpu";
    let s1_path = "/tmp/test_split - S1.pdf";
    let s2_path = "/tmp/test_split - S2.pdf";
    fs::write(input_path, multi_track_jianpu_input()).unwrap();
    let _ = fs::remove_file(s1_path);
    let _ = fs::remove_file(s2_path);

    let status = jianpu_cmd()
        .args(["generate", "pdf", input_path, "--split-tracks"])
        .status()
        .unwrap();

    assert!(status.success(), "generate pdf --split-tracks command failed");

    let s1_bytes = fs::read(s1_path).expect("S1 output file not found");
    assert!(s1_bytes.starts_with(b"%PDF"), "S1 output is not a valid PDF");

    let s2_bytes = fs::read(s2_path).expect("S2 output file not found");
    assert!(s2_bytes.starts_with(b"%PDF"), "S2 output is not a valid PDF");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(s1_path);
    let _ = fs::remove_file(s2_path);
}

#[test]
fn split_tracks_with_output_stem() {
    let input_path = "/tmp/test_split_out.jianpu";
    let s1_path = "/tmp/split_out - S1.pdf";
    let s2_path = "/tmp/split_out - S2.pdf";
    fs::write(input_path, multi_track_jianpu_input()).unwrap();
    let _ = fs::remove_file(s1_path);
    let _ = fs::remove_file(s2_path);

    let status = jianpu_cmd()
        .args(["generate", "pdf", input_path, "--output", "/tmp/split_out", "--split-tracks"])
        .status()
        .unwrap();

    assert!(status.success(), "generate pdf --output --split-tracks command failed");

    let s1_bytes = fs::read(s1_path).expect("S1 output file not found");
    assert!(s1_bytes.starts_with(b"%PDF"), "S1 output is not a valid PDF");

    let s2_bytes = fs::read(s2_path).expect("S2 output file not found");
    assert!(s2_bytes.starts_with(b"%PDF"), "S2 output is not a valid PDF");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(s1_path);
    let _ = fs::remove_file(s2_path);
}
```

- [ ] **Step 2: Run the tests to confirm they fail**

```bash
cargo test --test integration split_tracks 2>&1
```

Expected: compile error (unknown argument `--split-tracks`) or test failure.

- [ ] **Step 3: Add `split_tracks: bool` and update `output` to `Option<PathBuf>` (named) in each `GenerateFormat` variant**

In `src/main.rs`, update the `GenerateFormat` enum. The `output` field becomes `--output` explicitly (was positional before). Add `split_tracks`:

```rust
#[derive(Subcommand)]
enum GenerateFormat {
    Pdf {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        tracks: Vec<String>,
        #[arg(long)]
        split_tracks: bool,
    },
    Svg {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        tracks: Vec<String>,
        #[arg(long)]
        split_tracks: bool,
    },
    Midi {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        tracks: Vec<String>,
        #[arg(long)]
        split_tracks: bool,
    },
}
```

Note: `output` is now `--output` (named flag), not a positional argument. This is required so it can coexist with `--split-tracks` cleanly and to enforce stem-only usage.

- [ ] **Step 4: Add `collect_track_names` helper to `src/main.rs`**

Add this function after `filter_tracks`:

```rust
fn collect_track_names(score: &ast::grouped::Score) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut names = Vec::new();
    for measure in &score.measures {
        for part in &measure.parts {
            if let Some(name) = &part.name {
                if seen.insert(name.clone()) {
                    names.push(name.clone());
                }
            }
        }
    }
    names
}
```

- [ ] **Step 5: Update `run_generate` match arms to destructure `split_tracks` and implement the split logic**

For each arm (`Pdf`, `Svg`, `Midi`), the pattern is:

1. Destructure `split_tracks` from the arm.
2. Determine `effective_tracks` (explicit `--tracks` or `collect_track_names`).
3. If `split_tracks` and tracks exist, loop and generate one file per track; return early.
4. Otherwise, fall through to existing single-file logic (with `filter_tracks` using `tracks`, not `effective_tracks`).

**Full Pdf arm:**

```rust
GenerateFormat::Pdf {
    input,
    output,
    tracks,
    split_tracks,
} => {
    let mut score = parse_and_group(&input)?;
    let effective_tracks = if !tracks.is_empty() {
        tracks.clone()
    } else {
        collect_track_names(&score)
    };
    if split_tracks && !effective_tracks.is_empty() {
        let base = output_stem(&input, &[], output.as_deref());
        let base_name = base
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        for track in &effective_tracks {
            let mut score_clone = score.clone();
            filter_tracks(&mut score_clone, std::slice::from_ref(track));
            let row_height = score_clone.metadata.row_height;
            let note_number_width = score_clone.metadata.note_number_width;
            let pages = layout::layout(&score_clone, 595.0, 842.0);
            let svgs = renderer::render(&pages, row_height, note_number_width);
            let pdf_bytes = pdf::write_pdf(&svgs)?;
            let track_path = base
                .with_file_name(format!("{} - {}", base_name, track))
                .with_extension("pdf");
            write_file(&track_path, &pdf_bytes)?;
            println!("written to {:?}", track_path);
        }
        return Ok(());
    }
    if split_tracks {
        eprintln!("warning: --split-tracks given but score has no named tracks; generating single file");
    }
    filter_tracks(&mut score, &tracks);
    let row_height = score.metadata.row_height;
    let note_number_width = score.metadata.note_number_width;
    let pages = layout::layout(&score, 595.0, 842.0);
    let svgs = renderer::render(&pages, row_height, note_number_width);
    let pdf_bytes = pdf::write_pdf(&svgs)?;
    let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("pdf");
    write_file(&output_path, &pdf_bytes)?;
    println!("written to {:?}", output_path);
    Ok(())
}
```

**Full Svg arm:**

```rust
GenerateFormat::Svg {
    input,
    output,
    tracks,
    split_tracks,
} => {
    let mut score = parse_and_group(&input)?;
    let effective_tracks = if !tracks.is_empty() {
        tracks.clone()
    } else {
        collect_track_names(&score)
    };
    if split_tracks && !effective_tracks.is_empty() {
        let base = output_stem(&input, &[], output.as_deref());
        let base_name = base
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        for track in &effective_tracks {
            let mut score_clone = score.clone();
            filter_tracks(&mut score_clone, std::slice::from_ref(track));
            let row_height = score_clone.metadata.row_height;
            let note_number_width = score_clone.metadata.note_number_width;
            let pages = layout::layout(&score_clone, 595.0, 842.0);
            let svgs = renderer::render(&pages, row_height, note_number_width);
            let track_base = base
                .with_file_name(format!("{} - {}", base_name, track));
            for (i, svg) in svgs.iter().enumerate() {
                let path = if svgs.len() == 1 {
                    track_base.with_extension("svg")
                } else {
                    track_base.with_extension(format!("{}.svg", i + 1))
                };
                write_file(&path, svg.as_bytes())?;
                println!("written to {:?}", path);
            }
        }
        return Ok(());
    }
    if split_tracks {
        eprintln!("warning: --split-tracks given but score has no named tracks; generating single file");
    }
    filter_tracks(&mut score, &tracks);
    let row_height = score.metadata.row_height;
    let note_number_width = score.metadata.note_number_width;
    let pages = layout::layout(&score, 595.0, 842.0);
    let svgs = renderer::render(&pages, row_height, note_number_width);
    let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("svg");
    for (i, svg) in svgs.iter().enumerate() {
        let path = if svgs.len() == 1 {
            output_path.clone()
        } else {
            output_path.with_extension(format!("{}.svg", i + 1))
        };
        write_file(&path, svg.as_bytes())?;
        println!("written to {:?}", path);
    }
    Ok(())
}
```

**Full Midi arm:**

```rust
GenerateFormat::Midi {
    input,
    output,
    tracks,
    split_tracks,
} => {
    let mut score = parse_and_group(&input)?;
    let effective_tracks = if !tracks.is_empty() {
        tracks.clone()
    } else {
        collect_track_names(&score)
    };
    if split_tracks && !effective_tracks.is_empty() {
        let base = output_stem(&input, &[], output.as_deref());
        let base_name = base
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        for track in &effective_tracks {
            let mut score_clone = score.clone();
            filter_tracks(&mut score_clone, std::slice::from_ref(track));
            let midi_bytes = midi::write_midi(&score_clone);
            let track_path = base
                .with_file_name(format!("{} - {}", base_name, track))
                .with_extension("mid");
            write_file(&track_path, &midi_bytes)?;
            println!("written to {:?}", track_path);
        }
        return Ok(());
    }
    if split_tracks {
        eprintln!("warning: --split-tracks given but score has no named tracks; generating single file");
    }
    filter_tracks(&mut score, &tracks);
    let midi_bytes = midi::write_midi(&score);
    let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("mid");
    write_file(&output_path, &midi_bytes)?;
    println!("written to {:?}", output_path);
    Ok(())
}
```

- [ ] **Step 6: Run all tests to confirm they pass**

```bash
cargo test 2>&1
```

Expected: all 178+ unit tests pass, all integration tests pass including `split_tracks_generates_one_pdf_per_track` and `split_tracks_with_output_stem`.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs tests/integration.rs
git commit -m "feat: add --split-tracks flag to generate one file per track"
```
