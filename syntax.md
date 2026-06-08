# Jianpu Generator — `.jianpu` Syntax Reference

This document describes the input syntax accepted by **jianpu-generator** as implemented today. File extension: `.jianpu`.

---

## File structure

A `.jianpu` file has two sections:

```
[metadata]
…key = value fields…

[score]
…interleaved score content…
```

- `[metadata]` — **required**
- `[score]` — **required**
- Legacy `[score:Name]` / `[lyrics:Name]` sections are **not** supported.

Whitespace around `=` in metadata is optional. Metadata values may be quoted with `"`.

---

## Metadata

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `title` | yes | — | Piece title (rendered in header) |
| `author` | yes | — | Author name (rendered in header) |
| `subtitle` | no | none | Subtitle line |
| `parts` | no | `notes:` + `lyrics:` (unnamed single part) | Column order for `[score]` (see below) |
| `max columns` | no | `28` | Maximum grid columns per system line before wrapping |
| `row height` | no | `24` | Vertical spacing of one part row (pixels) |
| `label width` | no | `40` | Horizontal space reserved for part labels (pixels) |
| `note number width` | no | `8` | Horizontal space per note column (pixels) |

### `parts` declaration

Space-separated column tokens declaring the **positional order** of lines in each measure group:

```
parts = chord:main notes:Soprano lyrics:Soprano notes:Alto lyrics:Alto
```

| Token | Meaning |
|-------|---------|
| `notes:<name>` | A note part (unique name per notes column) |
| `lyrics:<name>` | Lyrics for the notes part with the same `<name>` |
| `chord:<name>` | A chord-symbol row (Nashville numbers) |

Rules:

- Each `lyrics:<name>` must pair with a preceding `notes:<name>` of the same name.
- A notes part may omit its `lyrics:` entry (instrumental part).
- Column order in `parts` is the column order in every measure group.

Example (multi-part vocal score with chords):

```
parts = chord:main notes:A1&T lyrics:A1&T notes:A2 lyrics:A2 notes:S1 lyrics:S1 notes:S2 lyrics:S2
```

---

## Score section — measure groups

The `[score]` body is split into **measure groups** by **blank lines**. Each group is exactly one bar (measure).

```
(bpm=92 key=C4 time=4/4 label="Verse 1")
1 - - -
_5 _5 _5 =5 =5 _5 _3 _2 _3~
白陽旗旛在大道盛宏

6m - - -
_3 _1~1 - _0 =1 =1
昌花花
```

### Group layout

1. **Optional directive line** — first line starting with `(` and ending with `)`
2. **Data lines** — one per `parts` column, in declaration order

Lines are trimmed; leading/trailing spaces on a line are ignored. A completely empty line separates measure groups (it is not a data line).

### Positional mapping

Data lines map to columns **by position**, not by content type inference:

| Index | `parts` column | Line content |
|-------|--------------|--------------|
| 0 | `chord:main` | chord symbols |
| 1 | `notes:A1&T` | note tokens |
| 2 | `lyrics:A1&T` | lyric text |
| 3 | `notes:A2` | note tokens |
| … | … | … |

You cannot skip a column in the middle — only **trailing** lines may be omitted (see [Implicit ditto](#implicit-ditto) below).

---

## Directive lines

An optional parenthesised first line sets global directives for that measure and onward (until overridden):

```
(bpm=92 key=C4 time=4/4 label="Verse 1")
```

| Directive | Example | Effect |
|-----------|---------|--------|
| `bpm=` | `bpm=120` | Tempo (beats per minute) |
| `key=` | `key=C4`, `key=F#3`, `key=Bb4` | Key signature (`1` = this note) |
| `time=` | `time=4/4`, `time=3/4` | Time signature |
| `label=` | `label="Verse 1"` | Section label rendered above the row group |

Rules:

- Multiple directives may appear in one `(...)` line, separated by whitespace.
- `label=` value must be a quoted string; empty labels are rejected.
- Directives apply to **all** parts. They are stored on the first notes part and propagate through grouping.
- `label` applies only to the measure where it is declared (does not persist to the next bar).
- `bpm`, `key`, and `time` persist until the next directive line overrides them.

Note names: `A` `B` `C` `D` `E` `F` `G`, with optional `#` or `b` accidental, followed by octave digit (e.g. `4`).

---

## Notes syntax

Note lines are whitespace-separated **tokens**. The `|` character is accepted but ignored (legacy bar separator).

### Pitch and rest

| Token part | Meaning |
|------------|---------|
| `1`–`7` | Scale degree (movable do) |
| `0` | Rest |

### Duration prefix

Duration is measured in **quarter-beats** (sixteenth-note units). In 4/4, one full beat = 4 quarter-beats; a full 4/4 bar = 16 quarter-beats.

| Prefix | Quarter-beats | Typical name (4/4) |
|--------|---------------|---------------------|
| *(none)* | 4 | Quarter note (one beat) |
| `_` | 2 | Eighth note |
| `=` | 1 | Sixteenth note |

Each token fills one **beat slot** by default; `-` (see below) extends the previous note/rest into the next beat slot.

### Octave dots

| Position | Meaning |
|----------|---------|
| Leading `.` | Raise octave (each `.` = one octave up) |
| Trailing `.` | Lower octave (each `.` = one octave down) |

Leading and trailing dots **cannot be mixed** on the same token.

Examples: `.1` (octave up), `1..` (two octaves down), `.._3` (up two octaves, eighth note).

### Modifiers

| Suffix | Meaning |
|--------|---------|
| `*` | Dotted (add half the base duration). Cannot combine with `=` (sixteenth) tokens. |
| `~` | Tie/slur attached to this note (connects to the next note) |

A standalone `~` token ties/slurs into the **preceding** note, including across a `-` extension:

```
6 - ~ 7 0      ← 6 extended one beat, then slurred into 7
```

The tokenizer splits on `~`, so `4~3~3` becomes three tokens: `4~`, `3~`, `3`.

### Extension

| Token | Meaning |
|-------|---------|
| `-` | Extend the previous note or rest by one beat (4 quarter-beats) |

Example: `1 - - -` is a whole note in 4/4 (one quarter-note token plus three extensions).

### Inline directives (notes row)

These tokens may also appear in a notes line (uncommon; usually placed in `(...)` directive rows instead):

| Token | Meaning |
|-------|---------|
| `bpm=N` | Tempo change |
| `1=<Note><octave>` | Key change, e.g. `1=C4`, `1=Bb4` |
| `N/N` | Time signature change, e.g. `4/4` |

### Measure validation

All note and rest durations in a row must sum to exactly the measure capacity. For time signature `N/D`:

```
measure capacity = N × (16 / D) quarter-beats
```

(e.g. 4/4 → 16, 3/4 → 12). Too few or too many quarter-beats is a parse error.

### Examples

| Token | Meaning |
|-------|---------|
| `1` | Quarter note on degree 1 |
| `_3` | Eighth note on degree 3 |
| `=5` | Sixteenth note on degree 5 |
| `_1*` | Dotted eighth note |
| `1~` | Quarter note tied to next |
| `6.` | Degree 6, one octave down |
| `0` | Quarter rest |
| `_0` | Eighth rest |
| `1* =1 =6.` | Mixed durations and octaves |

---

## Lyrics syntax

Lyrics lines are plain text tokenised into syllables:

| Script | Rule |
|--------|------|
| CJK (Chinese, Japanese, Korean) | Each character is one syllable |
| Latin | Space-separated words/syllables |

### Held syllable (`-` within lyrics)

A `-` **inside** a lyrics line marks the **preceding** syllable as *held* — it stretches across tied notes:

```
he llo - world     ← "llo" is held across the tied note
你 - - 好           ← first 你 is held across two tied notes
```

This is distinct from `-` on a notes line (duration extension) and distinct from `_` (see below).

### No-lyrics marker (`_`)

A lyrics line whose **entire** trimmed content is `_` means **zero syllables** for that part in this measure (instrumental bar):

```
1 2 3 4
do re mi fa

5 6 7 1
_
```

- `_` is valid **only** on lyrics columns.
- On notes or chord columns, `_` alone is a parse error (`_` is already the eighth-note duration prefix on notes lines).
- Ditto (`"`) copying a `_` source line also yields zero syllables.

### Empty lyrics

Empty lyrics lines are **not** allowed. Whitespace-only lines are treated as measure separators, not as empty lyrics. To express silence, write `_`.

### Lyrics–notes tally

In each measure, the number of lyric syllables must match the number of notes that take lyrics in the paired notes row:

- Each non-rest note head counts, except a **tie continuation** (same pitch immediately after a tied note, including across a bar line).
- Held-syllable markers (`-`) count as their own syllables — e.g. `你 - 好` is three syllables for three lyric slots.
- The `_` no-lyrics marker skips this check (zero syllables allowed regardless of notes).

Mismatch is a parse error, e.g. `lyrics has 3 syllables but notes need 4 in part 'Soprano'`.

---

## Chord syntax

Chord lines use Nashville number symbols. Duration works like notes: each token occupies one beat; `-` extends the previous chord.

| Token | Meaning |
|-------|---------|
| `0` | Chord rest |
| `-` | Extend previous chord one beat |
| `\|` | Ignored |
| `<symbol>` | Chord (see grammar below) |

### Chord symbol grammar

```
<chord>      ::= <degree> <accidental>? <triad>? <extension>? ("/" <bass>)?
<degree>     ::= 1–7
<accidental> ::= "#" | "b"
<triad>      ::= "m" | "o" | "+"
<extension>  ::= "M7" | "7"
<bass>       ::= <degree> <accidental>?
```

Parsing checks longest suffix first (`M7` before `7`; `m` before extension).

| Input | Meaning |
|-------|---------|
| `1` | I major |
| `1m` | I minor |
| `1o` | I diminished |
| `1+` | I augmented |
| `17` | I dominant 7th |
| `1M7` | I major 7th |
| `1m7` | I minor 7th |
| `1#m7` | I♯ minor 7th |
| `3b` | ♭III major |
| `1/5` | I major, 5 in bass (e.g. C/G) |
| `6m/5` | vi minor, 5 in bass (e.g. Am/G) |

Example:

```
1 - 6m -
_1 _1 _1 =1 =1 _1 6. _6~
```

---

## Ditto (`"`)

A line whose entire trimmed content is `"` means **same content as the closest preceding line of the same column type** within this measure group.

```
_5 _5 _5 =5 =5 _5 _3 _2 _3~    ← A1&T notes
白陽旗旛在大道盛宏               ← A1&T lyrics
"                                ← A2 notes  (= A1&T notes)
"                                ← A2 lyrics (= A1&T lyrics)
```

Resolution rules:

| Column type | Copies from |
|-------------|-------------|
| `notes:` | Closest preceding `notes:` line above |
| `lyrics:` | Closest preceding `lyrics:` line above |
| `chord:` | Closest preceding `chord:` line above |

- The `(...)` directive line is never a ditto source or target.
- Ditto chains resolve top-to-bottom (`"` copying `"` is fine).
- `"` with no preceding line of the same type in the group is an error.

---

## Implicit ditto

**Trailing** omitted lines are automatically treated as `"` when a same-type line already exists earlier in the measure group.

Verse example — three lines instead of nine:

```
1 - - -
_5 _5 _5 =5 =5 _5 _3 _2 _3~
白陽旗旛在大道盛宏
```

The omitted A2/S1/S2 notes and lyrics lines are padded as implicit ditto.

### Suffix-only omission

Because lines map **positionally**, you can only omit **trailing** columns. If a middle column would be ditto but later columns have explicit content, the middle `"` must still be written:

```
4* =4 =4 _4 _3 =1 =2~_2       ← A2 notes (explicit, diverges)
"                              ← A2 lyrics (still required — S1 follows)
6 - 5 -                       ← S1 notes (explicit)
一個                          ← S1 lyrics (explicit)
                               ← (omitted) S2 notes + S2 lyrics → implicit ditto
```

### Errors for omitted lines

| Situation | Result |
|-----------|--------|
| Omitted trailing line; same column type exists above | Implicit `"` |
| Omitted trailing line; no same-type precedent | Error — write content, `"`, or `_` (lyrics) |
| More data lines than `parts` columns | Error |
| Fewer than one data line per group | Error |

Explicit `"` lines remain valid (redundant when trailing omission would apply).

---

## Quick reference — special line forms

| Whole line | Column | Meaning |
|------------|--------|---------|
| `"` | notes, lyrics, chord | Ditto — copy preceding same-type line |
| `_` | lyrics only | No lyrics this bar |
| *(omitted, trailing)* | any | Implicit ditto if precedent exists |
| `(...)` | directive | Global bpm/key/time/label for this bar |

---

## Complete minimal example

```
[metadata]
title = "Demo"
author = "Author"
parts = chord:main notes:Melody lyrics:Melody

[score]

(bpm=120 key=C4 time=4/4 label="Verse")
1 - 4m 5
1 2 3 4
do re mi fa

1 - 4m 5
"
_
```

Bar 2: chord and notes are implicit ditto; lyrics explicitly marked `_` (no text this bar).

---

## Further reading

Design specs with additional rationale live in `docs/superpowers/specs/`:

- `2026-06-04-interleaved-syntax-design.md` — interleaved `[score]` format
- `2026-06-05-label-directive-design.md` — `label=` directive
- `2026-06-06-chord-track-design.md` — `chord:` columns
- `2026-06-06-ditto-input-dedup-design.md` — `"` ditto marker
- `2026-06-08-implicit-ditto-padding-design.md` — implicit ditto and `_` no-lyrics
