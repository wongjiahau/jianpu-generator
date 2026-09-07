# Jianpu Generator — `.jianpu` Syntax Reference

This document describes the input syntax accepted by **jianpu-generator** as implemented today. File extension: `.jianpu`.

---

## File structure

A `.jianpu` file has up to four sections, which may appear in any order:

```
# metadata
…key = value fields…

# parts
…track declarations…

# sequence
…comma-separated section labels…

# score
…interleaved score content…
```

- `# metadata` — **optional**
- `# parts` — **required**
- `# sequence` — **optional**
- `# score` — **required**
- Sections may appear in any order.
- Legacy `# score:Name` / `# lyrics:Name` sections are **not** supported.

Whitespace around `=` in metadata is optional. Metadata values may be quoted with `"`.

---

## Comments

`//` starts a comment that runs to the end of the line. It is recognized anywhere in the file — in the metadata, parts, or score sections, on its own line or trailing other content.

```
# metadata
title = "My Song"  // shown in the header

// this whole line is a comment
author = "Jane Doe"
```

A `//` inside a double-quoted string (e.g. `title = "http://example.com"`) is not treated as a comment.

---

## Metadata

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `title` | no | none | Piece title (rendered in header) |
| `author` | no | none | Author name (rendered in header) |
| `subtitle` | no | none | Subtitle line |
| `max_measures_per_system` | no | `4` | Maximum number of measures per system line before wrapping |
| `row_height` | no | `24` | Vertical spacing of one part row (pixels) |
| `note_number_width` | no | `8` | Horizontal space per note column (pixels) |
| `parts_list_columns` | no | `4` | Number of columns in the parts list header |
| `part_label_width_pt` | no | `40` | Fixed width (points) of the part-label column at the start of each system, shared by every system in the score regardless of how many measures/columns that system's music needs |
| `merge_duplicate_measures_across_parts` | no | `yes` | Score-wide default for whether identical measures from different parts are merged into a single row (`yes`/`no`); can be overridden from a given measure onward with the `merge_duplicate_measures_across_parts=` directive line — see [Directive lines](#directive-lines) |
| `hide_resting_parts` | no | `yes` | Score-wide default for whether an all-rest part is omitted from a measure where other parts have content (`yes`/`no`); can be overridden from a given measure onward with the `hide_resting_parts=` directive line — see [Directive lines](#directive-lines) |
| `hide_system_dividers` | no | `no` | Whether the horizontal divider line between systems is omitted (`yes`/`no`) |
| `directive_row_offset` | no | `0 0` | Translation `"x y"` (points) applied to every rendered directive row (bar number, section label, key, bpm, time signature), moving that row's text without affecting the layout of anything else. Not applied to the `# sequence` summary header. |

### Text styles

Every rendered text kind — `title`, `subtitle`, `author`, `sequence`, `part_legend`, `measure_number`, `section_label`, `page_number`, `part_label`, `lyrics`, `notes`, `chords`, `note_dash` — is configured with the same object-literal syntax:

```
<kind> = { font_size: N, horizontal_padding_pt: N, vertical_padding_pt: N, bold: yes/no, italic: yes/no, underline: yes/no, font_family: serif/sans_serif/monospace }
```

- `{` and `}` delimit the object; fields inside are separated by `,` and each field is `name: value`.
- Keys are unquoted. Any subset of the seven fields may be given, in any order; omitted fields fall back to their default.
- `title`, `subtitle`, and `author` are overloaded: a plain quoted string (`title = "My Song"`) sets the text content, while an object value (`title = { font_size: 32 }`) sets the style. Both may be set independently.

| Component | Meaning |
|-----------|---------|
| `font_size` | Font size in points |
| `horizontal_padding_pt` | Horizontal padding (points) reserved before the element, widening its column's spacing |
| `vertical_padding_pt` | Extra vertical padding (points) added above/below the element |
| `bold` | Whether the element renders bold (`yes`/`no`) |
| `italic` | Whether the element renders italic (`yes`/`no`) |
| `underline` | Whether the element renders underlined (`yes`/`no`) |
| `font_family` | Which of the app's three embedded font roles the element renders in: `serif`, `sans_serif`, or `monospace`. Accepted on every kind, including `notes`/`chords`/`note_dash` — those three kinds' glyph widths are re-measured against whichever font this resolves to, as part of column layout, so an override there can't desync rendered glyph widths from the widths layout computes. |

Defaults by kind:

| Kind | `font_size` default | `horizontal_padding_pt` default | `vertical_padding_pt` default | `bold` default | `italic` default | `underline` default | `font_family` default |
|------|----------------------|----------------------------------|--------------------------------|-----------------|-------------------|----------------------|-------------------------|
| `title` | `row_height * 1.5` | `0` | `0` | `no` | `no` | `no` | `serif` |
| `subtitle` | `row_height * 0.8` | `0` | `0` | `no` | `yes` | `no` | `serif` |
| `author` | `row_height * 0.6` | `0` | `0` | `no` | `no` | `no` | `serif` |
| `sequence` | `12` | `0` | `0` | `no` | `no` | `no` | `sans_serif` |
| `part_legend` | `row_height * 0.6` | `0` | `0` | `no` | `no` | `no` | `sans_serif` |
| `measure_number` | `10` | `0` | `0` | `no` | `no` | `no` | `sans_serif` |
| `section_label` | `12` | `0` | `0` | `yes` | `yes` | `no` | `sans_serif` |
| `page_number` | `row_height * 0.6` | `0` | `0` | `no` | `no` | `no` | `sans_serif` |
| `part_label` | `12` | `0` | `0` | `no` | `no` | `no` | `sans_serif` |
| `lyrics` | `row_height * 0.6` | `4` | `12` | `no` | `no` | `no` | `serif` |
| `notes` | `lyrics.font_size` | `4` | `0` | `no` | `no` | `no` | `monospace` |
| `chords` | `lyrics.font_size` | `4` | `0` | `no` | `no` | `no` | `monospace` |
| `note_dash` | `notes.font_size` | `4` | `0` | `no` | `no` | `no` | `monospace` |

`notes.horizontal_padding_pt` is also used for the multi-measure-rest bar's end insets and the tie/slur/underline/tuplet-bracket markings. `lyrics.vertical_padding_pt` is extra padding around a lyric syllable's hover/click-target box, added on top of the lyric font's own measured ascender+descender span. `part_label`'s reserved column width is the separate flat `part_label_width_pt` field (see the main metadata table above), not part of this object — it's a layout constant, not a text style component. `sequence`'s `bold`/`italic`/`underline`/`font_family` style the `# sequence` summary line's per-label spans (independent of `section_label`, even though a directive line's own inline `label="..."` renders with `section_label`'s style — see [Directive lines](#directive-lines)).

Example:

```
# metadata
title = "My Song"
title = { font_size: 32, bold: yes }
lyrics = { font_size: 18, vertical_padding_pt: 6, font_family: sans_serif }
notes = { horizontal_padding_pt: 6, italic: yes }
part_label_width_pt = 60
```

The old flat per-component keys (`lyrics_font_size`, `lyric_click_target_padding_pt`, `notes_horizontal_padding_pt`, etc.) are **not supported** — using one is reported as an unknown metadata field. `title.width_pt`/`part_label.width_pt` (the object-literal form) are also not supported: `title.width_pt` reserved a minimum box width for the rendered title but never actually affected layout, so it was removed outright; `part_label`'s width lives at the flat `part_label_width_pt` key instead, since it's a layout constant rather than a text style component.

---

## Parts section

One track per line. Blank lines are ignored.

```
<display-name> [[<abbreviation>]] = <column> [<column>…]
<display-name> [[<abbreviation>]] = follow[<target-abbreviation>]
```

### Left-hand side

| Form | Display name | Abbreviation (row label) |
|------|--------------|----------------------------|
| `Alto 1 & Tenor [A1&T]` | `Alto 1 & Tenor` | `A1&T` |
| `Melody` | `Melody` | `Melody` |
| `main` | `main` | `main` |

- Square brackets `[Abbr]` denote the **abbreviation** used as the row label and for `[Key]` prefix lines in the score.
- When brackets are omitted, the abbreviation equals the full display name.
- The display name is stored for future legend rendering; row labels use the abbreviation only.

### Right-hand side

| Pattern | Meaning | Score lines per measure |
|---------|---------|-------------------------|
| `chords` | Chord-symbol row | 1, plus 1 per positionally-attached lyric verse |
| `notes` | Notes (instrumental, or with lyrics) | 1, plus 1 per positionally-attached lyric verse |
| `percussion` | Unpitched GM drum hits | 1 |
| `follow[X]` | Inherit column layout from the part with abbreviation `X` | same as target |

An optional soundfont string `"<number>: <name>"` may follow the kind token (or `follow[X]` bracket) to select the MIDI timbre for that part. The number is the General MIDI program number (0–127). The `<name>` portion is a quoted string and may contain `=` and other characters (for example `"1: Grand = Piano"`). For example: `notes "52: Choir Aahs"` or `follow[A] "1: Grand Piano"`. If omitted on a concrete part, the default is program 52 (Choir Aahs). On a `follow[X]` part, the soundfont is inherited from the target when omitted.

For `percussion` parts, the soundfont number is instead a **GM percussion key** (e.g. `38` = Acoustic Snare, `36` = Bass Drum 1), not a GM program number — all percussion parts share MIDI channel 9 (the GM drum channel) and a single fixed GM Standard Kit program change; the number selects which drum sample within that kit each hit plays. The number is not checked against the melodic instrument catalog.

An optional volume suffix `XX%` (1–3 ASCII digits followed by `%`, parsed as an unsigned 8-bit number; values above 100 or 0 are accepted without error or clamping) may appear after the soundfont string (or after the kind token if there is no soundfont) to set the MIDI volume for that part. For example: `notes "52: Choir Aahs" 47%` or `notes 80%`. If omitted on a concrete part, the default is 100%. On a `follow[X]` part, volume is inherited from the target when omitted and may be overridden with an explicit `XX%` suffix.

An optional octave offset `+N` or `-N` (where N is 1–4) may appear anywhere on the right-hand side to shift every note in that part up or down by N octaves in MIDI output only. For example: `notes -1`, `notes +1`, `notes "5: Electric Guitar" -2`, or `follow[A] -1`. The offset does not change octave dots in the rendered SVG. If omitted on a concrete part, the default is 0. On a `follow[X]` part, the octave offset is inherited from the target when omitted and may be overridden with an explicit `+N` or `-N` suffix. Values outside ±4 emit a recoverable error and are clamped to ±4.

Rules:

- Duplicate abbreviations across tracks are an error.
- At least one track must be declared.
- `follow[X]` cannot be used for the first declared part.
- The target abbreviation `X` in `follow[X]` must refer to an already-declared part (declared before the follower).
- A `follow[X]` part that is not explicitly mentioned in a measure copies `X`'s content and is visually suppressed (row not rendered). This copies notes only — lyrics are never auto-copied to a follow part; a follow part gets a lyrics row only when a lyric line is positionally attached to it directly (see [Positional (unprefixed) lyrics lines](#positional-unprefixed-lyrics-lines)).
- A `follow[X]` part can be partially or fully overridden using `[Key]` prefix lines in the score.

Example (multi-part vocal score with chords):

```
# parts
main = chords
Alto 1 & Tenor [A1&T] = notes
Alto 2 [A2] = notes
Soprano 1 [S1] = notes
Soprano 2 [S2] = notes
```

Minimal single-part example:

```
# parts
Melody = notes
```

---

## Score section — measure groups

The `[score]` body is split into **measure groups** by **blank lines**. Each group is exactly one bar (measure).

```
bpm=92 key=C4 time=4/4 label="Verse 1"
[Melody] 5_ 5_ 5_ 5= 5= 5_ 3_ 2_ (3_)
[Melody] 白陽旗旛在大道盛宏

[Melody] 3_ (1_1) 0_- 1= 1=
[Melody] 昌花花
```

### Group layout

1. **Optional directive line** — first line containing at least one directive keyword (`bpm=`, `key=`, `time=`, `label=`, `merge_duplicate_measures_across_parts=`, or `hide_resting_parts=`)
2. **Data lines** — most data lines begin with a `[Abbrev]` prefix (see below); a bare line with no prefix is also allowed, as a **positional lyrics line** (see [Positional (unprefixed) lyrics lines](#positional-unprefixed-lyrics-lines))

Lines are trimmed; leading/trailing spaces on a line are ignored. A completely empty line separates measure groups (it is not a data line).

### Key-based part prefix (`[Abbrev]`)

Every data line must begin with `[Abbrev]` to route it to a specific part by abbreviation, including the first declared part:

```
[A2] 5 6 7 0
```

- Exactly one `[Key]` line may appear for a given part in a measure group (its single notes/chords slot); a second `[Key]` line for the same part is a `part [Key] has N lines but only 1 slot(s)` error — extra lyric verses must be written as bare, positionally-attached lines instead (see below).
- An unrecognised abbreviation is an error; the line is dropped.
- Parts not covered by any `[Key]` line use their `follow[X]` target's content when declared as such, or are filled with implicit rests/no-lyrics otherwise.
- A data line with no `[Abbrev]` prefix is a **positional lyrics line**, not an error: it attaches to whichever part's `[Key]` line most recently preceded it in this measure group (see [Positional (unprefixed) lyrics lines](#positional-unprefixed-lyrics-lines)). If no `[Key]` line precedes it in the measure group, it's a `score_line_missing_key_prefix` error, dropped as before.
- A measure group with zero valid keyed *and* zero positionally-attributed lines is an error (`measure_no_data_lines`).

**Row label when parts render as one unison row:** when two or more parts' compiled content ends up identical for a system (a system being one printed line of music, spanning however many measures were packed onto it), the renderer merges them into a single row, labeled by concatenating the merged parts' own abbreviations with a space (e.g. `S1 S2`).

**Row label omitted when a system boils down to one all-rest row:** a row label only earns its place by distinguishing one row from another sharing the same system (a system being one printed line of music, spanning however many measures were packed onto it). When every row that would otherwise appear in a system — after all merging above — collapses to a single row, and that row's content is entirely rest (whether one resting measure or a run collapsed into a wide multi-measure rest bar), there's nothing else in the system for the label to distinguish it from, so the label is omitted. This applies regardless of *why* it's the system's only row — a genuinely single-part score, or every other part being hidden by `hide_resting_parts=` — but not when that lone row actually sounds something, and not when more than one row shares the system even if all of them happen to be resting.

**Example — only part C plays, A and B are not-mentioned:**

```jianpu
# parts
A = notes
B = notes
C = notes

# score
time=4/4 key=C4 bpm=120
[A] 1 2 3 4

[C] 5 6 7 0
```

Measure 2: C plays `5 6 7 0`. A and B have no explicit lines → filled with `0` (rest) and marked not-mentioned (rows suppressed).

**Example — key-based lines in one measure with a follow part:**

```jianpu
# parts
A = notes
B = follow[A]
C = notes

# score
[A] 1 2 3 4
[C] 5 6 7 0
```

A: `1 2 3 4`. B: not mentioned → copies A's content via `follow`. C: `5 6 7 0`.

**Example — follow part with partial key override:**

```jianpu
# parts
Soprano [S] = notes
Alto [A] = follow[S]

# score
time=4/4 key=C4 bpm=120
[S] 1 2 3 4
do re mi fa
[A] 5 6 7 1
```

Soprano: notes=`1 2 3 4`, lyrics=`do re mi fa`. Alto: notes=`5 6 7 1` (key override), no lyrics — a follow part's notes-only override does not copy the target's lyrics; give Alto its own positionally-attached line (`[A] 5 6 7 1` followed by a bare lyric line) if it needs one.

---

## Directive lines

An optional first line of whitespace-separated `key=value` directives sets global values for that measure and onward (until overridden):

```
bpm=92 key=C4 time=4/4 label="Verse 1"
```

| Directive | Example | Effect |
|-----------|---------|--------|
| `bpm=` | `bpm=120` | Tempo (beats per minute) |
| `key=` | `key=C4`, `key=F#3`, `key=Bb4` | Key signature (`1` = this note) |
| `time=` | `time=4/4`, `time=3/4` | Time signature |
| `label=` | `label="Verse 1"` | Section label rendered above the row group |
| `merge_duplicate_measures_across_parts=` | `merge_duplicate_measures_across_parts=no` | Overrides the `#metadata` default from this measure onward (`yes`/`no`) |
| `hide_resting_parts=` | `hide_resting_parts=no` | Overrides the `#metadata` default from this measure onward (`yes`/`no`) |
| `break` | `break` | Forces a new system (line) to start at this measure |

Rules:

- Multiple directives may appear on one line, separated by whitespace.
- `label=` value must be a quoted string; empty labels are rejected.
- Directives apply to **all** parts. They are stored on the first notes part and propagate through grouping.
- `label` applies only to the measure where it is declared (does not persist to the next bar) — this is true for rendering purposes and whenever no `# sequence` section is present. When a `# sequence` section **is** present, each label additionally denotes a *span* of measures for playback-order purposes: see [`# sequence` — explicit playback order](#sequence--explicit-playback-order) below.
- `bpm`, `key`, and `time` persist until the next directive line overrides them.
- `merge_duplicate_measures_across_parts` and `hide_resting_parts` also persist until the next directive line overrides them; unset, they start from the `#metadata` value (or its default of `yes`) for the first measure.
- `break` is a bare keyword (no `=value`). It applies only to the measure it's written on — it does not persist — and forces that measure to start a new system, even if `max_measures_per_system` hasn't been reached yet. It's a no-op if the measure would already be first in its system (e.g. it's the very first measure of the score, or the previous measure already filled its system). Since it changes where a system boundary falls, a `break` measure is never absorbed into a collapsed multi-measure rest run that would otherwise start before it — see [Multi-measure rests](#multi-measure-rests).

### Rendering

When `time=` or `bpm=` changes on a measure, the generator may add a **directive row** above the bar-number / section-label row for that system line. Time signature and BPM appear once on that row (not on each part row), aligned with each measure’s note-start column. They do not shift notes or lyrics horizontally. If neither value changes on any measure in the line, the directive row is omitted.

Note names: `A` `B` `C` `D` `E` `F` `G`, with optional `#` or `b` accidental, followed by octave digit (e.g. `4`).

### `# sequence` — explicit playback order

A score may include an optional `# sequence` section — placed after `# parts` and before `# score` — that states the playback order directly, as a comma-separated list of section labels (the same labels set via `label="..."` on a measure's directive line):

```
# sequence
A, B, A

# score
time=4/4 key=C4 bpm=120 label="A"
1 2 3 4
label="B"
5 6 7 1
```

- Each entry in `# sequence` is a label declared with `label="..."` in `# score`; entries are separated by commas, and surrounding whitespace is trimmed.
- A label's **span** covers its measure and every following measure up to (but not including) the next `label="..."` measure, or through the end of the score if there is no following label. Above, `A` spans just its own measure, and `B` spans from its measure to the end of the score.
- Labels may be repeated in `# sequence` (e.g. `A, B, A`) to replay a span more than once.
- Each label must be declared **exactly once** in `# score`; declaring the same label on more than one measure is an error.
- Referencing a label in `# sequence` that was never declared in `# score` is an error; that entry is skipped and the rest of the sequence still resolves.
- `# sequence` only affects **MIDI/WAV playback order** — measures always render once, in written order, with normal bar numbers. However, SVG/PDF output does show the resolved order as a left-aligned line ("Sequence: A › B › A") on the first page, with a blank line of space above it, below the title/subtitle/author/part list. Each label is styled the same as an inline `label="..."` directive (bold, italic).

An entry may carry a `(-abbrev -abbrev ...)` suffix naming part abbreviations (as declared in `# parts`) to omit from that specific occurrence's playback — e.g. a chorus written once but replayed several times with a voice dropping out on later repeats:

```
# sequence
Verse, Chorus(-S -A2), Verse, Chorus(-A2), Chorus
```

- The suffix affects **only that occurrence**: here, the first `Chorus` omits Soprano and Alto 2, the second omits only Alto 2, and the third (unmarked) plays every part.
- An abbreviation that matches no declared part is an error; that abbreviation is dropped and the rest of the entry (and sequence) still resolves.
- The written-order rendering itself is unaffected — the score's written-out `Chorus` section always renders with every part, once, per the written-order rule above. However, the omissions **are** shown on the "Sequence: ..." summary line (SVG/PDF, first page), right after the label in plain (non-bold/non-italic) text: `Sequence: Verse › Chorus (-S -A2) › Verse › Chorus (-A2) › Chorus`. This is a reader-facing note only, telling a performer which voices tacet on which repeat — the underlying `Chorus` measures are not duplicated or altered.

An entry may instead carry an `(abbrev abbrev ...)` suffix — the same abbreviations, but with no leading `-` — naming the **only** parts to keep for that occurrence (every other declared part is omitted), for the opposite case: a chorus written once but soloed down to a subset of voices on a later repeat:

```
# sequence
Chorus, Verse, Chorus(S)
```

- Here the third `Chorus` plays only Soprano; every other declared part is silent for that occurrence. Naming every declared part (e.g. `Chorus(S A2 T)` when only those three parts exist) is equivalent to no suffix at all.
- A suffix cannot mix `-abbrev` and `abbrev` tokens (e.g. `Chorus(S -A2)`) — that's an error, and the entry falls back to no suffix (plays every part).
- Otherwise this suffix follows the same rules as the omit suffix above: an unknown abbreviation is an error and is dropped; the written-out section always renders every part; the summary line shows the suffix as written but without the dash, e.g. `Sequence: Chorus › Verse › Chorus (S)`.

---

## Notes syntax

Note lines are a sequence of **atoms** (notes, rests, chords, extensions, groups). Whitespace is optional between atoms and is ignored inside `(…)` groups.

Example: `((1 1) 5 5)` is equivalent to `((11)55)`.

### Pitch and rest

| Token part | Meaning |
|------------|---------|
| `1`–`7` | Scale degree (movable do) |
| `0` | Rest |

### Duration suffixes

Duration is measured in **quarter-beats** (sixteenth-note units). In 4/4, one full beat = 4 quarter-beats; a full 4/4 bar = 16 quarter-beats.

| Suffix | Quarter-beats | Typical name (4/4) |
|--------|---------------|---------------------|
| *(none)* | 4 | Quarter note (one beat) |
| `_` | 2 | Eighth note |
| `=` | 1 | Sixteenth note |

Suffix order is flexible (`1_,'` and `1',_` are equivalent).

### Tuplets

`N:{notes}` brackets `N` notes to be played in the time normally taken by a standard "against" count (3-in-2, 2-in-3, 5-in-4, ...); `N:M:{notes}` overrides that with an explicit `M`. The brace opens right after the count, not before it — `3:{1_1_1_}`, not `{3:1_1_1_}`.

```
3:{1_1_1_} 2_ 3_ 4_ 5_ 6_    an eighth-note triplet, then five plain eighth notes
5:4:{1=1=1=1=1=}             a quintuplet of sixteenth notes, explicit 5-in-4
```

| `N` | Implied `M` (against count) |
|-----|------------------------------|
| 2 | 3 |
| 3 | 2 |
| 4 | 3 |
| 5 | 4 |
| 6 | 4 |
| 7 | 4 |
| 9 | 8 |

Any other `N` has no standard implied ratio — omitting `:M` is a recoverable error ("tuplet ratio for N is ambiguous; use `{N:M:...}` to specify explicitly"); write `N:M:{notes}` instead.

The bracket must contain exactly `N` notes/rests/repeat-atoms (each counts once, same rule as `(…)` group note-counting) — a mismatch at the closing `}` is a recoverable error, though the notes present are still emitted and rendered.

Tuplets nest with `(…)` slur/tie groups in either direction:

```
(3:{1_1_1_} 2_) 3_ 4_ 5_ 6_    slur group wrapping a triplet
3:{(1_1_) 1_} 2_ 3_ 4_ 5_ 6_   triplet wrapping a slur group
```

Unlike `(…)` groups, a tuplet **cannot span lines**: an unclosed `{` at the end of a line is a hard parse error, not a cross-line continuation.

**Note:** the measure-capacity check (below) currently compares each tuplet's *written* (nominal, uncompressed) duration against the bar, not its actual rescaled duration — so a tuplet that only fits the bar once compressed/expanded (the whole point of writing one) can be misjudged as too short or too long at parse time. The triplet example above works because its notes' nominal durations, ignoring the tuplet, already sum to the bar's capacity on their own. Until this is fixed, keep a tuplet's *nominal* duration matching what the bar needs, or use it as a measure's only content.

### Octave markers

| Suffix | Meaning |
|--------|---------|
| `'` | Raise octave (each `'` = one octave up) |
| `,` | Lower octave (each `,` = one octave down) |

`'` and `,` **cannot be mixed** on the same note.

Examples: `1'` (octave up), `1,,` (two octaves down), `3_,'` (eighth note, up one octave).

### Accidentals (`#` / `b`)

Append `#` (sharp) or `b` (flat) immediately after a scale-degree digit to raise or lower the pitch by one semitone.

| Notation | Meaning |
|----------|---------|
| `7#`     | Scale degree 7, raised one semitone (leading tone sharpened) |
| `1b`     | Scale degree 1, lowered one semitone |
| `4#`     | Scale degree 4, raised one semitone (tritone) |

Accidentals can be combined with octave modifiers and all duration modifiers: `7#'` (sharp 7, octave up), `1b_` (flat 1, eighth note), `4#.` (sharp 4, dotted).

Rests (`0`) do not accept accidentals.

### Modifiers

| Suffix | Meaning |
|--------|---------|
| `.` | Dotted (add half the base duration). Cannot combine with `=` (sixteenth) notes. |
| `..` | Double-dotted (add half the base duration, then a quarter of it). Only valid when the base duration divides evenly by 4 (see below). |
| `-` | Extend the previous **note or rest** by one beat (4 quarter-beats) |
| `-.` | Extend the previous **note or rest** by one *dotted* beat (6 quarter-beats) — the natural beat of a compound meter (e.g. 9/8) |
| `-..` | Extend the previous **note or rest** by one *double-dotted* beat (7 quarter-beats) |
| `~` | Tie this note to the next note (same pitch and octave required) |

Example: `2 - - -` is a whole note in 4/4 (equivalent to `2---`). Likewise, `0 - - -` (or `0---`) is a whole rest.

`-.`/`-..` are standalone extension atoms (the dot(s) must be glued directly after the `-`, with no space) — they are not the same as a `-` suffix followed by a separate dotted/double-dotted note. In 9/8, `1. -. -.` is a note held across the full measure: a dotted quarter (`1.`, 6 quarter-beats) plus two dotted-beat extensions (6 + 6), totaling 18 quarter-beats. Similarly, `1.. -.. -.. -..` in 7/4 is a note held across the full measure: a double-dotted quarter (`1..`, 7 quarter-beats) plus three double-dotted-beat extensions (7 + 7 + 7), totaling 28 quarter-beats.

A second dot (`..`) adds a quarter of the base duration on top of the dot's own half — e.g. a double-dotted quarter note is `4 + 2 + 1 = 7` quarter-beats. This only lands on a whole quarter-beat when the base duration is a multiple of 4 (quarter notes, half notes, whole notes, ...). Applying `..` to a note whose base duration isn't a multiple of 4 — e.g. an eighth note, `1_..` (base duration 2) — is a recoverable error: the second dot is dropped and the note falls back to being singly dotted (`1_..` behaves like `1_.`, duration 3), with a diagnostic surfaced on the measure. Typing three or more dots (`1...`) is not a distinct feature — it's silently treated the same as two.

You can also attach dashes as suffixes on a note or rest (`2---`, `0---`). Both forms may be mixed in one measure. Repeated rests (`0 0`, `0 0 0 0`) remain equally valid — `0---` and `0 0 0 0` both produce a whole rest in 4/4.

Shorter rests still use `_`, `=`, or `.` on a single `0` (`0_`, `0=`, `0.`).

### Tie and slur groups

Parentheses connect notes with tie/slur arcs (happi123-style 连音符). A group may span measures: the opening `(` can appear at the end of one bar and the closing `)` at the start of the next.

| Form | Meaning |
|------|---------|
| `(12)` | Slur/tie from 1 into 2 |
| `(433)` | Slur chain across 4→3→3 |
| `(6-7)` | Note 6 extended one beat (`6-`), slurred into 7 |
| `111(1` … `2)345` | Cross-measure slur: `(1` opens in bar 1, `2)` closes in bar 2 |
| `(3= (2_1_))` | Nested groups: outer slur 3→2→1, inner slur 2→1 |

Groups may be **nested**: a `(…)` inside another `(…)` adds an inner tie/slur arc while the outer group still connects all enclosed notes. Each nested group must still contain at least 2 notes.

A group must contain **at least 2 notes** (counting notes across a cross-measure open/close). Single-note groups like `(5)` trigger a non-fatal **warning** (`group_too_few_notes`); rendering still proceeds.

### Tie (`~`)

`~` is written immediately after the octave modifier and before any duration modifiers:

```
4~---4---       tie two 4-beat notes
4'~4'           tie two high-4 quarter notes
4~.4.           tie quarter to dotted quarter
4~---4~---4---  chain of three tied notes
(4~---4--- 3)   tie inside a slur
```

Rules:
- Pitch, accidental, and octave must all match the next note — otherwise a recoverable error is emitted and the arc is suppressed.
- `~` on a rest is an error.
- `~` on the last note of the piece (no following note) is an error.
- Ties span freely across measure boundaries.
- Ties may appear inside slur groups `(…)`.

A tie differs from a slur `(…)` in that it requires identical pitch, and carries distinct semantic meaning (duration extension vs. phrasing).

### Repeat the last note/chord (`r`, bare `_`/`=`)

`r` repeats the last sounded pitch/chord as a fresh one-beat attack (a new note, not a tie/sustain). A bare `_` or `=` — one **not** glued directly after a digit — repeats it as an eighth-note or sixteenth-note attack respectively:

```
5 r r __        note 5, then three more 5s: a beat, a beat, and two eighths
5 0 r           note 5, a rest, then another beat of 5 (rests are skipped)
5~_             note 5 tied into its own eighth-note repeat
5__~5           note 5, an eighth-note repeat, tied out into the next note 5
```

Rules:
- "Last pitched note/chord" skips over intervening rests, and persists across measure boundaries (like ties/slurs).
- `r` never takes suffixes: `r_`, `r.`, `r'` are two atoms in sequence (`r` then a fresh atom), not `r` with a suffix glued on. Write repeats as multiple `r`s instead.
- Using `r`/`_`/`=` with no prior pitched note/chord on the track is a recoverable error; the token is dropped.
- A `~` glued directly after a repeat atom (`r`/`_`/`=`) ties that repeat into the following note, following the same rules as any other tie (matching pitch required, dangling tie is an error).

**Gotcha — maximal munch:** whitespace is cosmetic everywhere else in this grammar, but not here. `5_` (glued, no space) is unchanged: it's still note 5 shortened to an eighth note. Only a `_`/`=` that is *not* glued directly after a digit is a repeat atom — so `5 _` (with a space) repeats note 5's pitch as an eighth note, while `5_` does not. There are two exceptions, both because the glued character can't be read as a suffix of the preceding note in that position:
- Right after a tie: `5~_` glues fine (the `~` already claimed that spot).
- Right after another occurrence of the *same* suffix character: `5__` is note 5 shortened to an eighth note (first `_`) plus a repeated eighth-note attack (second `_`), not a no-op double-shorten. Likewise `5==` is a sixteenth note plus a repeated sixteenth, and this chains — `5___` is a note plus two repeats. Mixing different suffix characters still combines onto one atom as before: `5_=` is a single sixteenth note.

Adjacent digits without spaces also start new notes: `505` is three quarter notes; `(12)31` is a group plus two more notes.

Trailing duration may be omitted when the remaining measure beats extend the last note. In 4/4, `1` is equivalent to `1---`; `1 2` is equivalent to `1 2--`.

### Inline directives (notes row)

These tokens may also appear in a notes line (uncommon; usually placed in `(...)` directive rows instead):

| Token | Meaning |
|-------|---------|
| `bpm=N` | Tempo change |
| `1=<Note><octave>` | Key change, e.g. `1=C4`, `1=Bb4` (only when followed by A–G) |
| `N/N` | Time signature change, e.g. `4/4` |

Note: `1=` followed by a digit pitch (e.g. `1=,`) is a sixteenth note, not a key change.

### Measure validation

Note and rest durations in a row must fill the measure capacity. For time signature `N/D`:

```
measure capacity = N × (16 / D) quarter-beats
```

(e.g. 4/4 → 16, 3/4 → 12). Too many quarter-beats is a parse error. A shortfall extends the last note or rest when possible (so a lone `0` filling an empty measure is equivalent to `0---`). Otherwise it is a parse error.

#### Grouping validation (4/4 only)

In 4/4, the parser rejects rhythm spellings that cross metrical boundaries without exposing the split:

1. **Half-bar boundary:** after beat 1, no single note/rest may span from before beat 3 into beat 3 or beyond (quarter-beat position 8). Use a beam group such as `(2_ 2_)` or a tie instead of a single long value (e.g. `1. 2. 3_ 4_` is invalid; `1. (2_ 2_) 3_ 4_ 0_` is valid). Long notes/rests starting on beat 1 (including a fully extended `1` or `1---`) are allowed.
2. **Dotted-eighth tail:** a dotted eighth note/rest at the start of a beat must be followed immediately by a sixteenth note/rest filling the remaining sixteenth (e.g. `1_. 2= 3_ …`); `1_. 2_ 3_ 4_` is invalid (`2_.` is a dotted eighth, not an eighth).

Other time signatures skip these checks for now. Violations are diagnostics attached to the note (half-bar-boundary crossing is a **warning**; the dotted-eighth-tail rule is a **recoverable error**) — the file still renders.

### Multi-measure rests

This isn't new input syntax — it's automatic rendering behavior. When 2 or more consecutive measures are entirely rests (on every currently-visible part, after any `--tracks` filtering) and none of them carries its own directive (label, navigation marker, time signature/BPM/key change) or diagnostic, they render as a single wide rest bar showing the collapsed measure count, instead of one rest measure per bar. A single isolated all-rest measure still renders normally.

### Examples

| Token | Meaning |
|-------|---------|
| `1` | Quarter note on degree 1 |
| `3_` | Eighth note on degree 3 |
| `5=` | Sixteenth note on degree 5 |
| `1_.` | Dotted eighth note |
| `(12)` | Quarter notes 1 and 2, slurred/tied |
| `6,` | Degree 6, one octave down |
| `0` | Quarter rest |
| `0 0` | Half rest (two quarter rests) |
| `0 0 0 0` | Whole rest in 4/4 |
| `0_` | Eighth rest |
| `1. 1= 6=, (2_=2_)` | Mixed durations, octaves, and a slur group |

---

## Lyrics syntax

Lyrics lines are plain text tokenised into syllables:

| Script | Rule |
|--------|------|
| CJK (Chinese, Japanese, Korean) | Each character is one syllable |
| Latin | Space-separated words/syllables |

### Syllable break (`-` attached to a word)

A `-` **attached** to the end of a Latin syllable marks a word split across notes — the hyphen is part of the syllable text:

```
[Melody] 1 1 5 5
twin- kle twin- kle     ← "twinkle" split across two notes each
```

This is distinct from a **standalone** `-` surrounded by whitespace (held syllable, below).

### Held syllable (`-` within lyrics)

A `-` **inside** a lyrics line marks the **preceding** syllable as *held* — it stretches across tied notes:

```
[Melody] 4~4 3 2
he - world     ← "he" is held across the tied note
```

This is distinct from `-` on a notes line (duration extension) and distinct from `_` (see below).

### No-lyrics marker (`_`)

A lyrics line whose **entire** trimmed content is `_` means **zero syllables** for that part in this measure (instrumental bar):

```
[Melody] 1 2 3 4
do re mi fa

[Melody] 5 6 7 1
_
```

- `_` is valid **only** on lyrics columns.
- On notes or chord columns, `_` alone is a parse error (`_` is already the eighth-note duration prefix on notes lines).

### Empty lyrics

Empty lyrics lines are **not** allowed. Whitespace-only lines are treated as measure separators, not as empty lyrics. To express silence, write `_`.

### Lyrics–notes tally

In each measure, the number of lyric syllables must match the number of notes that take lyrics in the paired notes row:

- Each non-rest note head counts, except a **tie continuation** (same pitch immediately after a tied note, including across a bar line).
- Held-syllable markers (`-`) count as their own syllables — e.g. `你 - 好` is three syllables for three lyric slots.
- The `_` no-lyrics marker skips this check (zero syllables allowed regardless of notes).

Mismatch is a non-fatal **warning** (rendering continues, with empty-string syllables inserted for underflow), e.g. `[Soprano] lyrics underflow: ran out of syllables at syllable 3 (fewer syllables than notes)` or `[Soprano] lyrics overflow: 1 extra syllable(s) after all notes are consumed`.

### Positional (unprefixed) lyrics lines

Lyrics attach to a `notes`/`chords` part with a **bare (unprefixed)** data line: it attaches to whichever part's `[Key]` line most recently preceded it in the measure group. This works for any notes-bearing declared kind (`notes`, `chords`; not `percussion`, which has no lyrics pairing):

```
# parts
Melody = notes

# score
[Melody] 1 2 3 4
la la la la
```

- Consecutive bare lines after the same `[Key]` line become verses 1, 2, … , in order. Each verse renders as its own row directly under the notes row, in verse order, and each verse is tallied and tie-paired against the notes row independently — a verse can have its own `-` held syllables and `_` no-lyrics marker.
- Each verse row also gets its own label at the left margin, showing the part's abbreviation (e.g. `M`, same on every verse row) — clicking it, or including it in a click-and-click range selection, selects every syllable that verse sings across the system, the same way clicking a part's own label selects every note that part sounds.
- The number of verse lines is per-measure: one measure can have one verse while the next has two. A part's verse count changing from one measure to the next no longer forces a new system: a system's verse rows for a part are the union of every verse it has across the system's measures (see [Not-mentioned parts](#not-mentioned-parts) below), and a measure missing a verse renders that row blank for that measure only. A measure with no lyric line attached at all has zero verse rows for that part in that measure, not a blank placeholder verse.
- When two parts' `[Key]` lines both precede a bare line, it attaches to the **nearer** one only (the most recent `[Key]` line, not every preceding one). To attach the same words to two parts, write the line twice, once after each part's `[Key]` line.
- A repeated, explicitly `[Key]`-prefixed line (rather than a bare one) after a part's notes line is **not** a second verse — a fixed-schema part (`notes`, `chords`, `percussion`) only has one non-positional slot, so a second `[Key]`-prefixed line for the same part in one measure group is a `part [Key] has N lines but only 1 slot(s)` error. Extra verses must be written as bare, unprefixed lines.
- A bare line with no `[Key]` line above it yet in the measure has no part to attach to and is a `score_line_missing_key_prefix` error — a standalone caption line unrelated to a specific part's notes is not supported.
- One accepted trade-off: since a bare line following a `[Key]` line is always valid syntax, a composer who forgets a second part's `[Key]` prefix (meaning to write that part's notes) no longer gets an error — the line is silently absorbed as a positionally-attached lyrics line instead. Previously this was a hard `score_line_missing_key_prefix` error.

---

## Chord syntax

Chord lines use Nashville number symbols. Duration works like notes: each token occupies one beat; `-` extends the previous chord.

| Token | Meaning |
|-------|---------|
| `0` | Chord rest |
| `-` | Extend previous chord one beat |
| `<symbol>` | Chord (see grammar below) |

### Chord symbol grammar

```
<chord>      ::= <degree> <accidental>? <triad>? <extension>? ("/" <bass>)?
<degree>     ::= 1–7
<accidental> ::= "#" | "b"
<triad>      ::= "m" | "o" | "+" | "sus2" | "sus4" | "sus"
<extension>  ::= "M7" | "7"
<bass>       ::= <degree> <accidental>?
```

Parsing checks longest suffix first (`M7` before `7`; `sus2`/`sus4` before bare `sus`; `m` before extension). Bare `sus` (no digit) means `sus4`, matching standard chord-chart convention.

| Input | Meaning |
|-------|---------|
| `1` | I major |
| `1m` | I minor |
| `1o` | I diminished |
| `1+` | I augmented |
| `1sus2` | I suspended 2nd |
| `1sus4` | I suspended 4th |
| `1sus` | I suspended 4th (alias for `1sus4`) |
| `17` | I dominant 7th |
| `1M7` | I major 7th |
| `1m7` | I minor 7th |
| `1#m7` | I♯ minor 7th |
| `3b` | ♭III major |
| `1/5` | I major, 5 in bass (e.g. C/G) |
| `6m/5` | vi minor, 5 in bass (e.g. Am/G) |

### Duration suffixes

Chord heads accept the same suffixes as notes: `_`, `=`, `.`, and suffix `-`. Octave markers (`'`, `,`) are not allowed on chord lines.

### Repeating the last chord

`r` and bare `_`/`=` work the same way as on notes lines — see [Repeat the last note/chord](#repeat-the-last-notechord-r-bare-_) above: `1 r` repeats chord `1` for another beat, and `1 _` repeats it as an eighth note.

### Tie and slur groups

Parentheses work identically to notes lines. Spaces inside groups are ignored. Examples: `(1-6m-)`, `(1 - 6m -)`.

Example:

```
[chords] 1 - 6m -
[Melody] _1 _1 _1 =1 =1 1_ 6, (6_)
```

---

## Percussion syntax

Percussion lines carry unpitched GM drum hits. Duration works like notes: each token occupies one beat; `-` extends the previous hit.

| Token | Meaning |
|-------|---------|
| `0` | Rest |
| `x` | Hit |
| `-` | Extend previous hit one beat |

Duration suffixes (`_`, `=`, `.`), tie/slur groups (`(...)`), and the repeat-last-atom shorthand (`r`, bare `_`/`=`) work the same way as on notes lines — see [Notes syntax](#notes-syntax). Octave markers (`'`, `,`) and accidentals are not allowed on percussion lines, since hits have no pitch.

Example — snare and bass drum hitting simultaneously:

```
# parts
Snare = percussion "38: Acoustic Snare"
Kick = percussion "36: Bass Drum 1"

# score
[Snare] 0 x 0 x
[Kick] x 0 x 0
```

---

## Not-mentioned parts

When a part is **not mentioned** in a measure (no `[Key]` line covers it), it is filled with rests (`0`) or no-lyrics (`_`). If, after filling, that part's row is all rests for the measure **and at least one other part in the same measure has real content**, the row is **not rendered** for that measure — the vertical space is reclaimed and rows below move up. This suppression is controlled by the `hide_resting_parts` metadata field (default `yes`); set it to `no` to always render every part's row, even when it's all rests.

- A `follow[X]` part that is not mentioned copies `X`'s content (audio plays the same as X).
- A non-follow part that is not mentioned is filled with rests (`0`) or no-lyrics (`_`).
- Measures sharing a system line no longer need to render identical rows. A system's rows are the union of every part (and, per part, every verse) across all of its measures, in `[parts]` declaration order then verse order. A measure missing a row its system has (a part it doesn't mention, a resting part hidden via `hide_resting_parts`, or a verse it lacks) gets that row padded in as a full-measure rest or blank verse — it isn't clickable or selectable there, since no note/syllable actually sounds. Systems are otherwise packed purely by count, up to `max_measures_per_system` measures. Two things still start a new system line early: a `merge_duplicate_measures_across_parts=` change between measures — since that setting controls whether identical parts merge into one shared row, a change partway through would make the union ambiguous — and a `break` directive, which forces the boundary explicitly (see [Directive lines](#directive-lines)).

### Omitted lines — fill table

| Situation | Result |
|-----------|--------|
| Part not mentioned; declared as `follow[X]` | Copies X's content; row suppressed |
| Part not mentioned; no follow target; notes/chord slot | Silently filled with rests (`0`) |
| Part not mentioned; no follow target; lyrics slot | Silently filled with no-lyrics (`_`) |
| Data line missing `[Abbrev]` prefix, with a `[Key]` line earlier in the measure | Not an error — positional lyrics line (see [Positional (unprefixed) lyrics lines](#positional-unprefixed-lyrics-lines)) |
| Data line missing `[Abbrev]` prefix, with no preceding `[Key]` line in the measure | Error; line dropped (`score_line_missing_key_prefix`) |
| `[Key]` line with unrecognised abbreviation | Error; line dropped |
| No valid keyed lines in a measure group | Error (`measure_no_data_lines`) |

**Example — part B not mentioned:**

```jianpu
# parts
A = chords
B = notes

# score
[A] 1 2m 3 4

[A] 1 - - -
[B] 1 2 3 4
```

Measure 1: A plays `1 2m 3 4`, B is not mentioned → filled with rests, row suppressed.
Measure 2: A plays `1 - - -`, B plays `1 2 3 4`.

---

## Quick reference — special line forms

| Whole line | Column | Meaning |
|------------|--------|---------|
| `_` | lyrics only | No lyrics this bar |
| *(omitted)* | any | Rest fill or follow-target copy; row suppressed |
| `(...)` | directive | Global bpm/key/time/label for this bar |
| `[Abbrev] <content>` | notes, lyrics, chord | Key-based line targeting the named part by abbreviation |
| `<content>` (no `[Abbrev]`) | lyrics | Positional lyrics line — attaches to the nearest preceding `[Key]` line's part; an error if none precedes it (see [Positional (unprefixed) lyrics lines](#positional-unprefixed-lyrics-lines)) |

---

## Complete minimal example

```jianpu
# metadata
title = "Demo"
author = "Author"

# parts
Melody [M] = notes
Harmony [H] = follow[M]

# score

bpm=120 key=C4 time=4/4 label="Verse"
[M] 1 2 4 5
do re mi fa

[M] 1 2 4 5
_
[H] 3 5 6 7
do re mi fa
```

Bar 1: Melody plays `1 2 4 5` / `do re mi fa`. Harmony is not mentioned → copies Melody, row suppressed.  
Bar 2: Melody plays `1 2 4 5` / `_` (no lyrics). Harmony uses a `[H]` key line to override its notes, and its own positionally-attached line for lyrics.
