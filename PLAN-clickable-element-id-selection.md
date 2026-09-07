# Plan: resolve click-and-click selection by clickable-element ID, not live pixel coordinates

## Status

Phase 1's `Measure ↔ Measure`, `Note ↔ Note` (same part), `Lyric ↔
Lyric` (same part + verse), `PartLabel ↔ PartLabel` (any system), and
`LyricLabel ↔ LyricLabel` (any system, same verse) rows shipped:
`ClickableElementId`, `resolve_selection_range`/
`resolve_selection_range_response`, and `previewSelectionResolver.ts`'s
'measure'-, 'note'-, 'lyric'-, 'part-label'-, and 'lyric-label'-mode wiring
(each with the pixel fallback kept as a safety net for a combination this
plan hasn't given its own wasm arm yet — for 'note'/'lyric' mode, that's
`current` missing a click target of *either* type altogether, e.g. a
bar-line/gutter or empty-space point (see below — the cross-row case itself
is now ID-resolved); for 'part-label' mode, `current` missing every part
label's click target; for 'lyric-label' mode, that plus a different-verse
pair). That finishes every row Phase 1 originally scoped — the only rows
left are Phase 2's (label-mixed).

**Design principle, applies to every row in this plan, not just the ones
already ID-resolved**: click-and-click range resolution should have no
concept of a "system" at all. A system is a rendering/line-wrap
concern — where `Preview.tsx`'s layout happens to break a line — not a
domain one; nothing in `ClickableElementId` or `note_spans`/`lyric_spans`
needs it, and `measure_index` is already a single continuous count across
the whole piece, never reset per system. The cross-part `Note ↔ Note` arm
never had a system guard to begin with (see above) — it's the reference
shape every other cross-scope arm should match: derive a range purely from
the two endpoints' own index/measure fields, with no notion of "did this
cross a line break" anywhere in the rule. A plain click-and-click gesture
that happens to cross a system boundary should behave exactly like one that
doesn't; it should never need a modifier key, and it should never resolve to
*less* than what a same-system pair would.

`LyricLabel ↔ LyricLabel` was the first row brought in line with this:
made system-agnostic outright, rather than staying same-system-only, so a
plain click-and-click from a verse label in one system to the same verse's
label in another now resolves the same way a same-system pair always did,
just with the measure range widened to `min(anchor_start,
current_start)..max(anchor_end, current_end)` — no Cmd/Ctrl modifier
required; regression coverage: `lyric-label-range-select-crosses-system.feature`.
'lyric-label' mode's pixel-marquee fallback (`lyricLabelsInMarquee`) now
only gets reached for a different-verse pair.

`PartLabel ↔ PartLabel` got the same treatment next: dropped the
same-system guard on the plain (no-modifier) wasm arm, deriving the part
range from `min/max(sourcePartIndex)` (unchanged) and the measure range as
`min(anchor_start, current_start)..max(anchor_end, current_end)` — the
same shape `LyricLabel ↔ LyricLabel` uses — so a plain drag from one part's
label to that same part's (or a different part's) label in a later system
now resolves without a Cmd/Ctrl modifier, no matter which system either
endpoint sits in; regression coverage:
`part-label-range-select-crosses-system.feature`. This is deliberately
*not* the same as the pointer's old pixel sweep, which used to pick up
every label the drag rectangle happened to visually pass over (e.g.
Harmony's label sitting between two Melody labels stacked across systems);
the ID-based rule ranges only over the two endpoints' own
`sourcePartIndex`es, so that in-between Harmony label is no longer swept —
see `part-label-range-select-system-boundary.feature`'s updated scenario, which
documents this as the accepted tradeoff (same shape as the cross-part
`Note ↔ Note` arm's own staggered-rhythm tradeoff, noted above). The
Cmd/Ctrl-gated `'part-label-system'` mode (`partLabelsInMarqueeAcrossSystems`)
survives as a separate, coarser tool — "every part in every system the
gesture touches," which the plain drag's narrower per-endpoint-part-index
range doesn't replace — resolving this doc's previously-open Follow-up
question below in favor of "keep it distinct."

Phase 2's cross-part `Note ↔ Note` row also shipped (see below): with both
the same-part and cross-part `Note` arms in place, `resolve_selection_range`
can no longer return `Err` for a `Note ↔ Note` pair — but that only covers
pairs wasm can be asked to resolve in the first place, i.e. `current`
actually landing on a note. A commit that lines up on `previewSelectionResolver.ts`'s
`'note'`-mode branch briefly (2336791..8ec92d4) dropped its pixel-marquee
fallback entirely on the reasoning that it was now unreachable dead code;
that conflated "`Note ↔ Note` can't `Err`" with "`current` always resolves
to *some* note," which isn't true for a cross-row drag onto the lyric row
(`getNoteAtPoint` returns `undefined`, never reaching
`resolve_selection_range` at all) — a real regression, caught by
`note-lyric-cross-range-select.feature` going from passing to a 3-notes/
1-note failure. Fixed by restoring the marquee fallback for exactly the
`currentCell === undefined` case (not for a defined `currentCell` with a
non-`ok` response, which stays the genuinely-unreachable, logged-not-thrown
case). Lesson for the remaining rows below: "this pair-resolution arm can't
`Err` " is not the same claim as "every gesture in this mode reaches a
pair-resolution arm" — keep the pixel fallback for every path that doesn't,
even after a mode's ID-resolvable pairs are total.

Phase 2's `Note ↔ Lyric` cross-row also shipped (see below), in both
orderings (`Note` anchor/`Lyric` current and vice versa) and both scopes:
same-part ranges by `note_id` (shared numbering between a part's notes and
its lyrics), *not* `measure_index` — the cross-part `Note ↔ Note` arm's
measure-range pattern does not generalize to same-part cross-row, since a
measure routinely holds several notes (this row's own regression fixture is
one measure of four) and ranging by `measure_index` there would select the
whole measure as an all-or-nothing unit, far coarser than the old pixel
marquee or the sibling same-part `Note ↔ Note`/`Lyric ↔ Lyric` arms'
`note_id`-range rules. Cross-part cross-row has no shared `note_id` axis
across parts, so it does fall back to the measure-range pattern, accepting
the same coarseness tradeoff cross-part `Note ↔ Note` already accepts. Both
scopes restrict `lyric_cells` to the `Lyric` endpoint's own verse — the only
verse row a real drag actually swept — mirroring `LyricLabel ↔ LyricLabel`'s
single-verse scoping rather than `PartLabel ↔ PartLabel`'s "every verse"
(a part-label sweep has no verse of its own to scope by; a `Lyric` endpoint
does). With `Note ↔ Lyric` resolved, `previewSelectionResolver.ts`'s 'note'
and 'lyric' modes now only fall back to the pixel marquee when `current`
misses a click target of *either* type altogether (answering this row's
"Next question to answer" from before: yes for the pattern's general shape,
no for reusing `measure_index` unconditionally — see above).

`PartLabel ↔ PartLabel` was made system-agnostic next, the same way
`LyricLabel ↔ LyricLabel` already was (see the design-principle note
above) — the plain drag no longer needs a Cmd/Ctrl modifier to cross a
system boundary; see this doc's `PartLabel ↔ PartLabel` entry above for the
full writeup, including why it's *not* a byte-for-byte port of the old
pixel marquee's behavior (it ranges by `sourcePartIndex`, not by whatever
the drag rectangle visually swept) and why the Cmd/Ctrl `'part-label-system'`
mode survives as a separate tool rather than getting folded away.

Cross-verse (same part) and cross-part `Lyric ↔ Lyric` came next — the
syllable row's own version of the gap `LyricLabel ↔ LyricLabel` used to
have, distinct from it (the label-row fix doesn't touch this arm at all).
Today's shipped same-part-and-verse arm only has one axis (`note_id`); a
different verse or different part needs a second axis, so this is real
design work, not a mechanical port — worked out here before writing any
code:

- **Same part, different verse.** `note_id` numbering is shared across a
  part's verses the same way it's shared between a part's notes and
  lyrics (the fact the same-part `Note ↔ Lyric` arm already leans on) — so
  this doesn't need a fresh axis, just a second one alongside `note_id`:
  verse acts as a row index (mirroring how `PartLabel ↔ PartLabel` treats
  `sourcePartIndex` as a row index), `note_id` as the column. The rule:
  `verse_range = [min(anchor_verse, current_verse),
  max(anchor_verse, current_verse)]`, `note_id_range =
  [min(anchor_id, current_id), max(anchor_id, current_id)]`, select every
  `lyric_spans` entry in that part whose `verse` and `note_id` both fall in
  their range. This is a real (if small) 2-axis grid, but it needed none of
  the fuller row/column model the doc's Open Questions section still
  flags as unbuilt — same as `PartLabel ↔ PartLabel` and cross-part `Note ↔
  Note` before it, it falls out of each endpoint's own fields with no new
  plumbing. A plain drag today (pixel marquee) sweeps whatever rows its
  rectangle visually crosses, which — since verse rows render as stacked
  bands in increasing verse order under a part — already behaves like a
  contiguous verse range for a straight vertical drag; this rule matches
  that rather than diverging from it (unlike `PartLabel ↔ PartLabel`'s
  narrower-than-the-old-marquee tradeoff, which had a real behavior gap to
  justify diverging).
- **Cross-part (any verse pairing, including same-verse-different-part).**
  No shared `note_id` axis across parts, so this reuses the cross-part
  `Note ↔ Note`/`Note ↔ Lyric` arms' `measure_index`-range pattern for the
  column axis exactly (`measure_index` looked up per endpoint from
  `lyric_spans` by its own `(source_part_index, note_id, verse)`, same
  guard-if-missing caution as those arms) — answering this row's own "does
  cross-part reuse that pattern?" question: yes for the column axis. It
  does *not* reuse those arms' "no row restriction beyond part index"
  shape as-is, though: unlike a note (one row per part) or `Note ↔ Lyric`
  (one `Lyric` endpoint, so "the `Lyric` endpoint's own verse" is
  unambiguous), here *both* endpoints are `Lyric` and can each carry a
  different verse, so the row axis needs its own range as well —
  `verse_range` alongside `part_range`, the same verse-as-row-index idea
  the same-part case above uses. Final rule: `part_range =
  [min(anchor_part, current_part), max(...)]`, `verse_range =
  [min(anchor_verse, current_verse), max(...)]`, `measure_range =
  [min(anchor_measure, current_measure), max(...)]` (each measure looked up
  as above), select every `lyric_spans` entry whose part, verse, AND
  measure all fall in their range. When both endpoints share one verse,
  `verse_range` collapses to that single value, so this one rule also
  covers the same-verse-different-part case without a separate arm.
- **No note cells either way** — mirrors `Lyric ↔ Lyric`'s existing
  same-part-and-verse arm and `LyricLabel ↔ LyricLabel` (a lyric-only
  gesture doesn't reach into the note row), not `PartLabel ↔ PartLabel`
  (which does, because a part label represents the whole part, notes
  included, and a lyric endpoint never does).

Implemented as two new match arms in `selection_range_response` (guarded
same-part-different-verse first, cross-part-any-verse as the catch-all
fallthrough — the same guard-then-fallthrough shape the cross-part `Note ↔
Note` arm already uses relative to its same-part sibling), with Rust unit
tests for both plus their anchor/current-swapped order, and e2e coverage in
`lyric-range-select-crosses-verse.feature` /
`lyric-range-select-crosses-part.feature` (mirroring
`lyric-label-range-select-crosses-system.feature`'s conventions).
`previewSelectionResolver.ts`'s 'lyric'-mode comment no longer needs its
"additionally for its own remaining cross-verse/cross-part gap" caveat —
this closes it out, along with the `same_part_different_verse_lyric_pair_
returns_err`/`cross_part_lyric_pair_returns_err` unit tests, both of which
flip from asserting `Err` to asserting the resolved cells above.

**Label-mixed combinations** — the last item this plan's Follow-up flagged
as not yet scheduled — are now designed and shipped. Answer to "does every
label-mixed pair need the fuller row/column model the doc's Open Questions
section flags as unbuilt": **no — none of the five do.** Every one reduces
to the same endpoint-field-derived `part_range`/`measure_range` shape the
already-shipped cross-part `Note ↔ Note`, `Note ↔ Lyric`, `PartLabel ↔
PartLabel`, and cross-part `Lyric ↔ Lyric` arms already use — the fuller
model stays unbuilt, and there's no combination left that would need it.
Worked out per-pair, not assumed to be one shape, because a label endpoint
contributes different things depending on what it's paired with:

- **A `Note` or `PartLabel` endpoint always contributes to `note_cells`.**
  Both inherently represent "a point (or span) in the note row"; pairing
  either with anything else still selects notes across the combined
  `part_range`/`measure_range`.
- **A `Lyric` or `LyricLabel` endpoint always contributes to `lyric_cells`,
  restricted to *its own* `verse`.** A real verse is always known on that
  side (whether from a single syllable's `verse` field or a lyric label's),
  so there's never a reason to fall back to "every verse" *when a genuine
  lyric-side endpoint is present* — that fallback (`PartLabel ↔ PartLabel`'s
  cross-part "every verse" arm) only ever existed because *neither* side of
  that pair carries verse information at all.
- **`Note ↔ PartLabel` is the one pair where neither side carries verse
  info.** It therefore reuses `PartLabel ↔ PartLabel`'s own rule verbatim,
  treating the `Note` endpoint as a degenerate single-measure "label" for
  its own part (`measure_start == measure_end == its own measure_index`,
  looked up from `note_spans` the same way the cross-part `Note ↔ Note` arm
  already does): `part_range`/`measure_range` from both endpoints' fields,
  `note_cells` always, `lyric_cells` only when the pair's two parts differ
  (mirroring `PartLabel ↔ PartLabel`'s existing "same part → no lyric row"
  rule) and then unrestricted by verse (mirroring that same arm's "every
  verse" cross-part behavior) — same shape, same rationale, just with one
  side supplied by a note instead of a label.
- **`Lyric ↔ LyricLabel` is the one pair where *both* sides carry verse
  info**, so — mirroring cross-part `Lyric ↔ Lyric`'s own `verse_range`
  idea rather than requiring an exact match the way the plain `LyricLabel ↔
  LyricLabel` arm still does — it ranges over `verse_range =
  [min(anchor_verse, current_verse), max(...)]` alongside `part_range`/
  `measure_range` (the `Lyric` side's single measure treated as its own
  `[measure_index, measure_index]` span, looked up from `lyric_spans`).
  Collapses to a single verse when both sides happen to share one, so this
  one arm also covers that case without a separate rule. `note_cells` stays
  empty always, mirroring `Lyric ↔ Lyric`/`LyricLabel ↔ LyricLabel` — a
  lyric-only gesture never reaches into the note row.
- **`Note ↔ LyricLabel`, `Lyric ↔ PartLabel`, and `PartLabel ↔ LyricLabel`
  are all the same "mixed" shape**: one side is note-contributing (`Note` or
  `PartLabel`), the other is lyric-contributing with its own verse (`Lyric`
  or `LyricLabel`) — `PartLabel ↔ LyricLabel` isn't a sixth special case,
  it's this same pattern with both sides already being labels. `part_range`/
  `measure_range` combine both endpoints' fields (a `Note`/`Lyric` side's
  single measure treated as its own `[measure_index, measure_index]` span,
  looked up as above); `note_cells` = every `note_spans` entry in range,
  always (not gated by part-match, unlike `Note ↔ PartLabel` — the
  lyric-contributing side here is never a duplicate of the note-contributing
  one the way two `PartLabel`s can be, so there's no degenerate-click case
  to guard against); `lyric_cells` = every `lyric_spans` entry in range
  restricted to the lyric-contributing side's own `verse`, always. One
  shared Rust helper (`resolve_note_like_lyric_like_range`) backs all three
  pairs.

Implemented as five new match-arm groups in `resolve_selection_range_response`
(`Note ↔ PartLabel`, `Lyric ↔ LyricLabel`, and the three sharing the mixed
helper), each with both orderings, plus Rust unit tests for all five
(including anchor/current-swapped order) and e2e coverage in
`note-partlabel-range-select.feature`, `lyric-lyriclabel-range-select.feature`,
`lyric-partlabel-range-select.feature`, `note-lyriclabel-range-select.feature`,
and `partlabel-lyriclabel-range-select.feature`. `previewSelectionResolver.ts`'s
'note'/'lyric'/'part-label'/'lyric-label' mode branches now also try
`getPartLabelAtPoint`/`getLyricLabelAtPoint`/`getNoteAtPoint`/`getLyricAtPoint`
(whichever the mode didn't already try) when resolving `current`, so a second
click landing on a label from a note/lyric anchor (or vice versa) reaches
wasm instead of falling straight to the pixel marquee.

This closes out every `(anchor-type, current-type)` combination Phase 1 and
Phase 2's tables ever scoped. It does *not* mean every pixel-based fallback
in the click-and-click gesture is now dead code, though — see below for why
the cleanup this doc's old Follow-up section anticipated turns out to be
narrower than "delete it all."

**Final cleanup — narrower than originally anticipated, and why.** The old
Follow-up wording above ("once every remaining combination... delete the
pixel-marquee fallback path entirely... rather than leaving it in place as
permanent dead code") was written before this session's `'note'`/`'lyric'`
mode work established a load-bearing precedent: *every* mode's commit path
still needs a pixel fallback for `current` missing a click target of *any*
recognizable type at all (a bar-line/gutter/empty-space point) — there is no
`ClickableElementId` for such a point, ever, so it was never a "combination"
in Phase 1/2's tables to begin with, and can't be closed out by adding more
wasm arms. `'note'`/`'lyric'` mode already keep `applyNoteRangeSelection`/
`applyLyricRangeSelection`'s marquee fallback for exactly this case (see
their entries above); `'part-label'`/`'lyric-label'` mode now do the same —
once every *other* type `current` can resolve to (`Note`, `Lyric`,
`PartLabel`, `LyricLabel`) is ID-resolved via the arms above, the only
trigger left for `partLabelsInMarquee`/`lyricLabelsInMarquee`'s pixel path in
`resolveSelection`'s `'part-label'`/`'lyric-label'` branches is that same
off-target case, mirroring `'note'`/`'lyric'` mode exactly. So: **no function
gets deleted** — `cellsInMarquee`/`applyNoteDragHighlights`/
`applyLyricRangeHighlights` (`previewRangeHighlights.ts`), `partLabelsInMarquee`/
`lyricLabelsInMarquee` (`previewLabelRangeHighlights.ts`), and
`applyNoteRangeSelection`/`applyLyricRangeSelection` (`previewRangeSelection.ts`)
all stay, doing real work for the off-target case in both the commit path and
`usePreviewClickSelection.ts`'s live hover-preview loop (a different,
still-in-scope pixel-based job this plan never touches, per this session's
task instructions). What *is* now unreachable, and left in place with an
updated comment rather than deleted (out of scope for this pass — flagged for
a future small cleanup, not silently ignored): the `currentCell !== undefined
&& currentCell.sourcePartIndex === anchorCell.sourcePartIndex` `noteId`-range
branches inside `applyNoteRangeSelection`/`applyLyricRangeSelection`
(`noteCellsInNoteIdRange`/`lyricCellsInVerseNoteIdRange`) when called from the
*commit* path specifically — that call site only ever reaches them with an
already-off-target `current` (wasm having already claimed every on-target
case first), though they're still live code from the *hover* loop's own
direct call, which can land on a matching cell mid-drag. `partLabelsInMarqueeAcrossSystems`
stays untouched either way — it backs the distinct, intentionally-kept
coarser `'part-label-system'` tool (see above), not a not-yet-ported
combination.

**Step 7 (the hover-preview loop) shipped.** `usePreviewClickSelection.ts`'s
`document`-level `mousemove` listener — the hand-rolled per-mode min/max-union
and marquee dispatch this plan's step 7 originally deferred — is gone,
replaced by `mouseover`/`mouseout` listeners on the preview container that
delegate to the same `resolveSelection` (`previewSelectionResolver.ts`) the
commit path already used. The hovered element's own `ClickableElementId` is
read directly off its `data-*` attributes via the new
`clickableElementIdFromElement` (`clickableElementId.ts`, the shared TS
mirror this plan's step 5 originally introduced inline in
`previewSelectionResolver.ts`) through `event.target.closest('[data-tag]')`
— no `elementFromPoint`/`elementsFromPoint` pixel scan on the hover path at
all, except 'measure' mode, which keeps calling the point-based
`getMeasureAtPoint` during hover (the one documented exception: a
note/lyric's click-target rect is a DOM *sibling* of the measure/bar-line
group underneath it, not its ancestor, so `closest()` can't reach it —
fixing that needs the separate, smaller `getMeasureAtPoint` rect-scan fix
below). `PreviewAnchorState`'s `noteCellAtAnchor`/`lyricCellAtAnchor`/
`anchorSystem` fields collapsed into one `anchorId: ClickableElementId` per
mode (mirroring `resolveSelection`'s own anchor-read), stashed once at
anchor time in `previewClickHandler.ts` rather than re-derived from
`dragState.anchor`'s pixel coordinates at resolve time.

## Goal

Replace pixel-geometry range resolution in the preview's click-and-click
gesture (`web/src/components/previewSelection.ts`,
`previewRangeSelection.ts`, `previewLabelRangeHighlights.ts`,
`previewLabelSelection.ts`) with a single Rust/wasm function that computes a
selection from two **clickable-element IDs**, not two `{x, y}` points. TS
keeps exactly one job it can't give up — turning a mouse pixel into an
element ID via `elementFromPoint`/dataset reads — and stops doing the
second, riskier job of re-deriving "which cells lie between these two
points" from live coordinates.

## Why: the bug class this eliminates

Two bugs already required point patches this session:
[325e150](:/) resolved same-part note/lyric ranges by `noteId` order instead
of a pixel marquee (fixing a scrolled/stale-anchor bug), and `e1191d7`
added `suppressNextRevealRef` to stop an auto-scroll from fighting a user's
manual scroll mid-gesture. Both are symptoms of the same root cause: parts
of the range-resolution logic still depend on live `{x, y}` coordinates
being valid *at commit time*, when in a click-and-click (not held-drag)
gesture arbitrary scrolling can happen between the two clicks.

The remaining pixel-dependent paths — `applyNoteDragHighlights`/
`applyLyricRangeHighlights`'s cross-part marquee fallback,
`partLabelsInMarquee`/`partLabelsInMarqueeAcrossSystems`,
`lyricLabelsInMarquee`, and `getMeasureAtPoint`'s manual rect scan — all
have the same latent bug class, just not yet hit by a regression test.
Moving range resolution to operate on IDs instead of coordinates makes the
whole class structurally impossible for the modes it covers, rather than
patching instances one at a time.

## Design

### `ClickableElementId`

A tagged union mirroring exactly what TS already reads off `data-*`
attributes today (no new ID scheme to invent) — new type in
`crates/jianpu-wasm/src/note_selection_types.rs` (or a new
`selection_range_types.rs`, since it's shared by note+lyric+label
resolution, not note-only):

```rust
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ClickableElementId {
    Note { source_part_index: usize, note_id: usize },
    Lyric { source_part_index: usize, note_id: usize, verse: usize },
    Measure { measure_index_start: usize, measure_index_end: usize },
    PartLabel {
        source_part_index: usize,
        measure_index_start: usize,
        measure_index_end: usize,
    },
    LyricLabel {
        source_part_index: usize,
        verse: usize,
        measure_index_start: usize,
        measure_index_end: usize,
    },
}
```

No `Tsify`/`into_wasm_abi` needed — like `NoteCellIn`, this only ever comes
*in* from JS as part of a `JsValue`, decoded with `serde_wasm_bindgen`. The
matching hand-written TS type (`ClickableElementId` in a new
`web/src/components/clickableElementId.ts`) is a discriminated union on
`kind`, built directly from the same dataset fields
`getNoteAtPoint`/`getLyricAtPoint`/`getPartLabelAtPoint`/
`getLyricLabelAtPoint`/`getMeasureAtPoint` already parse — those functions'
*return types* change from bespoke `NoteCell`/`LyricCell`/etc. to
`ClickableElementId`, everything else about them (the `elementFromPoint`
walk-up) is unchanged. `SectionLabel` is deliberately excluded: a
section-label click doesn't anchor a range gesture at all (see
`handleAnchorClick`'s `onSectionLabelClick` branch), it's a direct
navigation callback.

### The wasm function

```rust
/// Resolves the click-and-click range between two clickable elements —
/// the ID-based replacement for pixel-marquee range resolution. Pure
/// grouping over already-fetched `note_spans`/`lyric_spans` (see
/// `group_note_selection`'s doc comment for why callers must call this on
/// the main thread with cached spans, not re-parse `source`).
#[wasm_bindgen]
pub fn resolve_selection_range(
    raw_note_spans: JsValue,
    raw_lyric_spans: JsValue,
    raw_anchor: JsValue,
    raw_current: JsValue,
) -> ResolveSelectionRangeResponse
```

`ResolveSelectionRangeResponse` mirrors `GroupNoteSelectionResponse`'s
shape: `Ok { note_cells: Vec<NoteCellOut>, lyric_cells: Vec<LyricCellOut> }
| Err` (`Err` for a malformed `JsValue`, matching the `unwrap_or_default`
pattern already used for `note_spans`/`selected_cells`).

### Per-combination semantics (the real design work)

This is where today's pixel marquee is *implicitly* the spec — moving to
IDs means making each combination's rule explicit. Split into two phases:

**Phase 1 — combinations with an existing unambiguous, already-ID-shaped
rule** (each one already has a TS function doing exactly this walk over
`noteSpans`/`lyricSpans`, just gated behind "does `current` happen to land
on a matching-type target" pixel checks):

| anchor | current | Rule | Existing TS logic to port |
|---|---|---|---|
| Note (part A) | Note (part A) | `noteId` range within part A | `noteCellsInNoteIdRange` |
| Lyric (part A, verse V) | Lyric (part A, verse V) | `noteId` range within (A, V) | `lyricCellsInVerseNoteIdRange` |
| Measure | Measure | measure-index range, both note+lyric cells in range | `noteCellsInMeasureRange` / `lyricCellsInMeasureRange` |
| PartLabel (system S) | PartLabel (system S) | every part between anchor's and current's `sourcePartIndex` in system S | `partLabelsInMarquee` (drop the pixel-intersection test, keep the index-range logic) |
| PartLabel (any system) | PartLabel (any system), Cmd/Ctrl | every part in every system from anchor's to current's | `partLabelsInMarqueeAcrossSystems` (drop pixel test, keep "which systems are touched" → now a system-index range instead of a rect-intersection test) |
| LyricLabel (system S, verse V) | LyricLabel (system S, verse V) | mirrors PartLabel same-system | `lyricLabelsInMarquee` |

Every row above already has zero real geometric content once you look past
the pixel gate — the marquee's *only* job in these cases is "confirm
`current` matches anchor's type/scope," which an ID's own discriminant and
fields answer directly, with no geometry.

**Phase 2 — combinations with no existing crisp non-geometric rule** (today
these fall through to the raw pixel marquee, which sweeps a rectangle
across whatever it happens to cover):

- ~~Note ↔ Note, different parts (diagonal cross-part drag)~~ — **shipped.**
  Answer to this row's own open question: "every note whose beat-position
  falls in the swept measure range, across every part between anchor's and
  current's part index" — derived purely from each ID's own fields plus a
  `note_spans` lookup (each endpoint's own `measure_index`, looked up by
  `(source_part_index, note_id)`), same pattern as `PartLabel ↔ PartLabel`'s
  "derive from `sourcePartIndex` alone." No new row/column model needed —
  see `resolve_selection_range_response`'s cross-part `Note` arm in
  `selection_range.rs`. Deliberately coarser than the old pixel marquee in
  a staggered-rhythm case (different note density per part in the same
  measure range) — accepted tradeoff, not a bug (see that arm's doc
  comment). The remaining rows below still need the fuller model.
- ~~Note ↔ Lyric (cross-row drag)~~ — **shipped**, in both orderings and
  both same-part/cross-part scope. Answer to the "Next question" this row
  used to pose (below): the cross-part `Note ↔ Note` arm's pattern
  generalizes for the *cross-part* scope only, unchanged — range over each
  endpoint's own `measure_index`, selecting both note and lyric cells (the
  latter scoped to the `Lyric` endpoint's own verse). It does *not*
  generalize to the *same-part* scope: a measure routinely holds several
  notes, so ranging by `measure_index` there is far coarser than the old
  pixel marquee — same-part instead reuses the same-part `Note ↔ Note`/
  `Lyric ↔ Lyric` arms' `note_id`-range rule directly (shared numbering
  between a part's notes and its lyrics), no measure lookup needed. See
  `resolve_selection_range_response`'s `Note ↔ Lyric` arms (guarded
  same-part first, cross-part fallthrough) and `resolve_note_lyric_range`
  in `selection_range.rs`. No new row/column model needed for this row
  either, same as cross-part `Note ↔ Note`.
- ~~Anything mixed with a label (Note ↔ PartLabel, etc.)~~ — **shipped.**
  Turned out *not* to need the row/column model this bullet originally
  guessed it might: every one of the five pairs (`Note ↔ PartLabel`,
  `Lyric ↔ LyricLabel`, `Note ↔ LyricLabel`, `Lyric ↔ PartLabel`,
  `PartLabel ↔ LyricLabel`) reduces to the same endpoint-field-derived
  `part_range`/`measure_range` shape the cross-part `Note ↔ Note`/
  `Note ↔ Lyric` rows above already established, worked out per-pair rather
  than assumed — see this doc's Status section for the full writeup and
  `resolve_selection_range_response`'s new arms in `selection_range.rs`.

### `getMeasureAtPoint`'s rect-scan (separate, smaller fix)

Not part of the ID-resolution function itself, but the same investigation
surfaced it: the manual `getBoundingClientRect` loop in
`previewSelection.ts:187` exists only because adjacent measures' click
targets touch flush, making `elementFromPoint` ambiguous on the shared
boundary pixel. Fix at the render layer instead: have
`grid_layout`/`renderer::new_renderer::render_measure_click_target` give
the boundary pixel to exactly one neighbor (e.g. right-measure-inclusive),
mirroring how note/lyric click targets never have this ambiguity. Once
that's true, `getMeasureAtPoint` collapses to the same
`elementFromPoint`-based `getCellAtPoint` every other hit-test already
uses, and `getBarLineMeasureAtPoint`'s prev/next rect scan can likely
simplify too. Worth doing either just before or just after this plan, but
is an independent, smaller commit — don't bundle it into the ID-resolution
migration.

## Migration steps

1. `crates/jianpu-wasm/src/selection_range_types.rs` (new): `ClickableElementId`,
   `ResolveSelectionRangeResponse`, `NoteCellOut`/`LyricCellOut` (or reuse
   existing output types if `note_selection_types.rs`/`lyric_selection_types.rs`
   already have equivalents — check before adding new ones).
2. `crates/jianpu-wasm/src/lib.rs`: add `resolve_selection_range`, following
   `group_note_selection`'s pattern exactly (decode two `JsValue` span
   arrays + two `JsValue` IDs via `serde_wasm_bindgen::from_value`).
3. Rust-side implementation (new module, e.g.
   `crates/jianpu-wasm/src/selection_range.rs`): Phase 1 table above, one
   match arm per `(anchor, current)` discriminant pair; anything not in the
   table returns `Err` (signalling "caller, use the pixel-marquee fallback"
   — see step 6).
4. Rust unit tests (separate file per this repo's convention, e.g.
   `crates/jianpu-wasm/src/selection_range_tests.rs`): table-driven,
   `(note_spans, lyric_spans, anchor, current) -> expected cells`, covering
   every Phase 1 row above plus its reverse (`current`/`anchor` swapped) and
   an out-of-scope Phase 2 pair (asserting `Err`, i.e. "caller must fall
   back"). No browser needed — this is the main win over today's e2e-only
   coverage for these combinations.
5. `web/src/components/clickableElementId.ts` (new): the TS mirror type,
   plus updating `getNoteAtPoint`/`getLyricAtPoint`/`getPartLabelAtPoint`/
   `getLyricLabelAtPoint` (in `previewSelection.ts`/
   `previewLabelSelection.ts`) to return `ClickableElementId` instead of
   their current bespoke cell/hit types. `PreviewAnchorState`
   (`previewAnchorState.ts`) stores a `ClickableElementId` for `anchor`
   instead of a `AnchorPoint`/`NoteCell`/etc. pairing — collapses several of
   its variants' redundant fields (e.g. 'note' mode no longer needs both
   `anchor: AnchorPoint` and `noteCellAtAnchor` — the ID *is* the cell).
6. `previewClickHandler.ts`/`previewSelectionResolver.ts`: `resolveSelection`
   calls `resolve_selection_range(noteSpans, lyricSpans, anchorId,
   currentId)` first; on `Err` (Phase 2 combination), fall back to the
   existing pixel-marquee path unchanged. This keeps Phase 2 behavior
   byte-for-byte identical while Phase 1 combinations stop touching pixels
   at all.
7. Mousemove hover-preview loop (`usePreviewClickSelection.ts`): switch its
   per-mode branches to call the same resolver with `(anchorId,
   hoveredId)` on every tick where the hovered point resolves to a
   `ClickableElementId` at all — same call as the commit path, just live.
   Where the pointer is over unrecognized space, keep highlighting the
   last-known ID's own resolution (no worse than today, which currently
   just stops updating the highlight past a hit-test miss).
8. Delete now-dead code:
   `noteCellsInNoteIdRange`/`lyricCellsInVerseNoteIdRange`
   (`previewRangeSelection.ts`), the same-system marquee/index logic inside
   `partLabelsInMarquee`/`lyricLabelsInMarquee` (keep only what Phase 2
   still needs), `noteCellsInMeasureRange`/`lyricCellsInMeasureRange`
   call sites that duplicated what `resolve_selection_range` now owns.

## Rollout / risk management

- Ship Phase 1 as one commit per row of the table above (or a small batch),
  each swappable independently behind the `Err`-falls-back-to-marquee
  seam in step 6 — never a big-bang rewrite of the whole gesture at once.
  Start with 'measure' mode (simplest, already fully index-based in TS
  today, lowest risk) to validate the wasm plumbing end-to-end before
  touching 'note'/'lyric'/label modes.
- Existing e2e specs (`note-range-select-crosses-page.feature`,
  `note-range-select-crosses-system.feature`,
  `note-lyric-cross-range-select.feature`,
  `part-label-click-selects-notes.feature`, etc.) stay as regression
  coverage throughout — they should keep passing unchanged since the
  *contract* (which cells end up selected) isn't changing, only how it's
  computed. If any of them needs `stableBoundingBox`-style flake-hardening
  as an artifact of pixel-based coordinate computation (like
  `note-range-select-crosses-page.steps.ts`'s poll from `e1191d7`), that's
  a signal the underlying scenario is exactly the kind of stale-coordinate
  bug this plan targets — investigate rather than just re-polling harder.
- `suppressNextRevealRef` (`e1191d7`) is untouched by this plan — it fixes
  a *when-does-reveal-scroll-fire* bug, not a range-resolution bug. Keep it.

## Open questions to resolve before starting

- Where should `ClickableElementId` and `resolve_selection_range` actually
  live — `note_selection_types.rs` is note-specific; this plan assumes a
  new shared module, confirm naming against this repo's existing
  `*_selection_types.rs` convention.
- ~~Phase 2's row/column model is real design work...~~ — **resolved for
  cross-part `Note ↔ Note` and `Note ↔ Lyric`**: neither needed the fuller
  row/column model after all. Cross-part `Note ↔ Note`'s answer was exactly
  the parenthetical above — "every note whose beat-position falls in the
  swept measure range, across every part between anchor's and current's
  part index" — Phase-1-portable the same way `PartLabel ↔ PartLabel` is
  (derive from each ID's own fields plus a `note_spans` lookup, no new
  plumbing). `Note ↔ Lyric` reuses that same measure-range pattern for its
  cross-part scope, but *not* for same-part (a measure holds several notes,
  so same-part instead ranges by `note_id`, reusing `Note ↔ Note`/`Lyric ↔
  Lyric`'s own same-part rule). See Phase 2's table above. **Resolved for the
  label-mixed combinations too** — none of the five needed the fuller
  row/column model either; see this doc's Status section for the per-pair
  writeup. With every combination now closed, this question has no more open
  cases left to check.
