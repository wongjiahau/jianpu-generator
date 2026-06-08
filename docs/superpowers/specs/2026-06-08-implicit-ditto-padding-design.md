# Implicit Ditto Padding and Explicit No-Lyrics Marker

## Summary

Make trailing `"` ditto lines optional in multi-part measures by padding omitted trailing lines with implicit ditto markers. Disallow implicit empty lyrics; introduce `_` as an explicit whole-line "no lyrics" sentinel on lyrics columns.

## Motivation

Scores like `彌勒淨土鄉.jianpu` repeat the same six trailing `"` lines in every verse measure where all lower parts match the lead voice. Authors must write these lines even though the content is fully determined by ditto rules. Omitting them today pads with empty strings, which is wrong for notes/lyrics ditto and ambiguous for lyrics silence.

## Goals

1. **Optional trailing ditto** — when remaining parts would all be `"`, omit those lines; the parser pads them as implicit ditto.
2. **Explicit divergence** — when a part differs (e.g. chorus), write real content; only omit trailing lines that would have been `"`.
3. **No implicit empty lyrics** — an omitted lyrics line never means "no lyrics"; authors must write `_`.
4. **Backward compatible** — explicit `"` lines continue to work.

## Non-Goals

- Mid-measure line omission (only trailing lines may be omitted; column order is positional).
- A no-lyrics marker on notes or chord columns.
- Output/rendering changes (input-side only, same as the original ditto feature).

## Syntax

### Implicit ditto (new)

Omitted **trailing** lines in a measure group are treated as if the author had written `"`, provided a line of the same column type already appears earlier in that group.

```
1 - - -                        ← chord
_5 _5 _5 =5 =5 _5 _3 _2 _3~   ← A1&T notes
白陽旗旛在大道盛宏              ← A1&T lyrics
                               ← (omitted) A2 notes, A2 lyrics, S1 notes, S1 lyrics, S2 notes, S2 lyrics
                               ← all padded as implicit ditto
```

Partial divergence (chorus-style) — write explicit lines, omit only **suffix** dittos. A `"` that precedes explicit content in later columns must stay explicit (positional mapping; only trailing columns can be omitted):

```jianpu
4* =4 =4 _4 _3 =1 =2~_2       ← A2 notes (explicit)
"                              ← A2 lyrics (still required before S1 diverges)
6 - 5 -                       ← S1 notes (explicit)
一個                          ← S1 lyrics (explicit)
                              ← (omitted) S2 notes, S2 lyrics → implicit ditto
```

### Explicit ditto (unchanged)

A line whose entire trimmed content is `"` still means ditto. Redundant once implicit padding exists, but valid.

### No-lyrics marker (new)

A lyrics line whose entire trimmed content is `_` means **zero syllables** for that part in this measure — instrumental / no text.

```
1 2 3 4
do re mi fa

5 6 7 1
_                             ← explicit: no lyrics this bar
```

`_` is recognized only on **lyrics** columns. On notes or chord columns it is a parse error (notes already use `_` as a duration prefix).

Ditto chains inherit the marker: if the source lyrics line is `_`, a `"` (explicit or implicit) copying it also yields zero syllables.

### Disallowed

| Input | Result |
|-------|--------|
| Empty lyrics line (`""` after trim) | Parse error: lyrics line cannot be empty; use `_` for no lyrics |
| Omitted trailing line with no same-type precedent in group | Parse error: must write content, `"`, or `_` (lyrics) |
| `_` on notes or chord column | Parse error |

## Padding Rules

In `validate_and_pad_group_lines`, replace the current `unwrap_or_default()` (empty string) logic:

```
for each column index i in 0..parts.len():
  if data_lines[i] present → use it
  else if same column type exists at some index j < i in this group → pad with "\""
  else → error: "expected <type> line for column <name>"
```

Column type matching follows existing desugar rules: Notes, Lyrics, and Chord are matched separately. The directive line `(…)` is not a ditto source.

### Minimum line count

Relax the minimum from `notes_cols_count` to **1** data line per measure group. Only trailing omission is supported; lines map positionally to columns.

## Pipeline

```
collect_groups()
   ↓
validate_and_pad_group_lines()   ← CHANGED: implicit " padding + stricter errors
   ↓
desugar_groups()                 ← unchanged: resolves " to preceding same-type content
   ↓
process_bar_group()              ← CHANGED: lyrics "_" → empty syllables; reject empty lyrics lines
   ↓
group / layout / render         ← unchanged
```

## Error Cases

| Condition | Error |
|-----------|-------|
| Omitted trailing line, no preceding line of same column type in group | `expected <notes\|lyrics\|chord> line for <part name>` |
| Empty lyrics line (after trim) | `lyrics line cannot be empty; use '_' for no lyrics` |
| `_` on notes or chord column | `'_' is only valid on lyrics lines` |
| More data lines than `parts.len()` | unchanged: too many lines |

## Migration

### `彌勒淨土鄉.jianpu`

Verse measures: remove six trailing `"` lines per bar (keep chord + lead notes + lead lyrics).

Measures that already mix explicit content and `"` (chorus): remove only the trailing `"` lines.

No change needed where lyrics are real text.

### Single-part scores using omitted lyrics

Previously, bar 2+ could omit the lyrics line and get empty lyrics via padding. Authors must now write `_` on those bars. Update `allows_missing_trailing_lyrics_line_in_subsequent_bars` and `layout/mod.rs` test helper comments accordingly.

## Tests

### New

- Multi-part verse bar with 3 lines resolves identically to 9 lines with explicit dittos
- Multi-part partial chorus bar: mixed explicit + omitted trailing dittos
- Omitted trailing line with no precedent → error
- Lyrics line `_` → zero syllables appended for that part
- Empty lyrics line → error with hint to use `_`
- `_` on notes column → error
- Ditto copying `_` source → zero syllables
- Explicit `"` still resolves (backward compat)

### Updated

- `allows_missing_trailing_lyrics_line_in_subsequent_bars` → use `_` instead of omission; rename to reflect explicit marker

## Files to Change

| File | Change |
|------|--------|
| `src/parser/score/interleaved_parser.rs` | Padding logic, minimum line count, lyrics `_` handling, empty lyrics rejection, tests |
| `src/desugar.rs` | Optional: tests for implicit padding integration (may be covered by interleaved_parser tests) |
| `src/layout/mod.rs` | Update test helper comment; any test using omitted lyrics → `_` |
| `彌勒淨土鄉.jianpu` | Remove redundant trailing `"` lines |

## Open Questions

None — symbol choice is `_` (underscore). Can revisit if authors find it confusing alongside notes duration `_`.
