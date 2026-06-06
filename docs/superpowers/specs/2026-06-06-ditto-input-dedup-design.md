# Ditto Marker for Input Deduplication

## Summary

Add a `"` ditto token that can appear on any score line (notes or lyrics) to mean "same content as the immediately preceding line of the same column type." Resolution happens in a new `desugar` stage that sits between the parser and the grouper. Downstream stages never see a ditto token.

## Motivation

In multi-part scores, it is common for several voice parts to share identical notes or lyrics across many measures. Writing out the same content repeatedly is error-prone and makes the `.jianpu` file hard to edit. The ditto marker removes this duplication at the source level without changing the rendered output.

## Syntax

A score line whose entire content is a single double-quote character `"` is a ditto line.

```
_5 _5 _5 =5 =5 _5 _3 _2 _3~    ← A1&T notes
白陽旗旛在大道盛宏               ← A1&T lyrics
"                                ← A2 notes  (resolves to A1&T notes)
"                                ← A2 lyrics (resolves to A1&T lyrics)
"                                ← S1 notes  (resolves to A2 notes = A1&T)
"                                ← S1 lyrics (resolves to A2 lyrics = A1&T)
"                                ← S2 notes  (resolves to S1 notes)
"                                ← S2 lyrics (resolves to S1 lyrics)
```

When parts diverge partway through a group, only the lines that differ need actual content:

```
6* =6 =6 _6 _5 =3 =2~_2         ← A1&T notes
你一個神秘的地                   ← A1&T lyrics
4* =4 =4 _4 _3 =1 =2~_2         ← A2 notes  (different)
"                                ← A2 lyrics (= A1&T)
6 - 5 -                         ← S1 notes  (different)
一個                             ← S1 lyrics (different)
"                                ← S2 notes  (= S1)
"                                ← S2 lyrics (= S1)
```

## Pipeline

```
parse()    → ParsedScore  (may contain Ditto nodes in notes/lyrics/chord lines)
   ↓
desugar()  → ParsedScore  (no Ditto nodes; all resolved to concrete content)
   ↓
group()    → grouped::Score
   ↓
layout / midi / wav  (unchanged)
```

The parser recognises `"` as a `Ditto` variant and stores it in the AST. The desugarer is a small, independently testable pass that replaces each `Ditto` with a copy of the preceding line of the same column type. Error messages for invalid ditto usage are emitted by the desugarer, not the parser.

## Resolution Rules

- A `"` on a **notes** line copies the content of the closest preceding **notes** line within the same measure group.
- A `"` on a **lyrics** line copies the content of the closest preceding **lyrics** line within the same measure group.
- A `"` on a **chord** line copies the content of the closest preceding **chord** line within the same measure group.
- A ditto at the start of a measure group with no preceding line of that type in the same group is an error.
- Ditto chains are allowed: `"` copying a `"` is fine because resolution is applied sequentially top-to-bottom, so by the time the second `"` is resolved the first has already been replaced with real content.

## Scope

- Input side only. All downstream stages (layout, renderer, MIDI, WAV) are unchanged — they never see a ditto token.
- No output rendering changes in this feature.
- Per-track output (`--split-tracks`) is unaffected; since ditto is resolved before grouping, individual tracks always see their full content.

## Error Cases

| Condition | Error |
|-----------|-------|
| `"` appears as first notes/lyrics line in a measure group with no prior line of that type | Parse error: "ditto with no preceding \<type\> line" |
| `"` used on a chord line with no prior chord line | Same |
