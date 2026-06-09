# Grouping Validation Design (4/4, v1)

## Goal

Reject `.jianpu` scores whose rhythm spelling crosses metrical boundaries without exposing the split via beam groups. Start with two hardcoded 4/4 rules derived from concrete examples; generalize to other meters and rules later.

## Non-goals (v1)

- Auto-normalization or warnings-only mode
- Time signatures other than 4/4
- Rest-spelling rules beyond the dotted-eighth-tail case
- Weak-beat placement, tie/group mixing, chord tracks
- Layout/beaming changes (rendering is unchanged)

## Approach

Add a dedicated validator module called after measure padding, when durations are final and source spans are still available.

```
tokenize → validate_and_pad_beats → validate_4_4_grouping → continue
```

### Why a separate file

- Rules stay readable and testable in isolation
- `interleaved_parser.rs` does not grow further
- Future meters/rules extend one module instead of scattering checks

## Module

**File:** `src/grouping.rs` (new)

**Public API:**

```rust
pub fn validate_measure_grouping(
    events: &[Spanned<ScoreEvent>],
    time_sig: TimeSignature,
) -> Result<(), JianPuError>
```

- No-op (returns `Ok`) when `time_sig` is not 4/4
- Called from `validate_and_pad_beats` in `interleaved_parser.rs` after implicit padding

**Registration:** add `mod grouping;` in `src/lib.rs` (or `main.rs` / crate root as appropriate)

## Rules (hardcoded 4/4)

Quarter-beat grid: 16 per bar. Beat boundaries at 0, 4, 8, 12. **Half-bar boundary at 8** (between beats 2 and 3).

Walk timed events (notes and rests) left-to-right, tracking `pos` (quarter-beats from measure start). Skip `Extension`, `TieMarker`, and other non-timed events.

### Rule 1 — Half-bar boundary

**Reject** any timed event where `pos < 8` and `pos + duration > 8`.

This catches spellings like `1. 2. 3 4` where `2.` (dotted quarter, 6 qb) starts at 6 and crosses the beat-2/beat-3 midpoint.

**Accept** `1. (2_ 2) 3 4` because each eighth is checked individually (6–7 and 8–9); neither single event spans across 8.

**Error message:** `note/rest crosses the half-bar boundary (beat 2→3); use a beam group or tie to show the split`

**Error span:** the offending event's span.

### Rule 2 — Dotted-eighth tail within a beat

When a **dotted eighth** note or rest appears (`dotted == true` and `duration == 3`) at a **beat-aligned** position (`pos % 4 == 0`):

- It consumes 3 qb of a 4-qb beat, leaving exactly **1 qb** before the next beat.
- The **next timed event** must be the opening note/rest of a `(…)` group (`group_membership > 0` on that note).
- All timed events belonging to that group (until `group_continuation` returns to 0 on the closing note) must sum to **exactly 1 qb**.

This catches `1_. 2_. 3_ 4_` and the rest equivalent `0_. 1_ …`.

**Accept** `1_. (2=) 3_ 4_ …` — a parenthesized sixteenth group filling the 1-qb tail. (The author's example used `(2= 2_)`; implementation tests should verify the exact canonical tail against parsed durations and adjust if the example was informal.)

**Error message:** `dotted eighth must be followed by a beam group filling the remaining sixteenth`

**Error span:** the dotted-eighth event (rule 2a) or the next event if it fails to open a group (rule 2b).

## Event inspection details

Use existing `ParsedNote` / `ParsedRest` fields:

| Field | Use |
|-------|-----|
| `duration` | Boundary arithmetic (after padding) |
| `dotted` | Identify dotted-eighth tail case |
| `group_membership` | Detect `(…)` group opening |
| `group_continuation` | Walk grouped tail duration |

Timed events are `ScoreEvent::Note` and `ScoreEvent::Rest` only.

## Error handling

- Reject with `JianPuError::new(span, message)` — same pattern as other parse validation
- No new error kind enum required for v1

## Testing

Table-driven unit tests in `src/grouping.rs`:

| Input (4/4 notes line) | Expected |
|------------------------|----------|
| `1. 2. 3 4` | Error — rule 1 |
| `1. (2_ 2) 3 4` | OK |
| `1_. 2_. 3_ 4_` | Error — rule 2 |
| `1_. (2=) 3_ 4_ 5_ 6_` | OK (confirm tail group sums to 1 qb) |
| `0_. 1_ 2_ 3_` | Error — rule 2 (dotted eighth rest) |

Use the existing parse-and-validate test harness pattern (`parser::parse` + full pipeline, or call `validate_measure_grouping` directly on padded events).

## Documentation

Add a short **Grouping validation** subsection under **Measure validation** in `syntax.md`:

- Only 4/4 for now
- The two rules in plain language
- Violations are parse errors

## Future work (not in this spec)

- Parameterize beat/half-bar boundaries from time signature
- Additional rules from My Music Theory (rests, compound time)
- Chord-track validation if needed
