import type { ClickableElementId } from './clickableElementId'
import type { AnchorPoint } from './previewRangeHighlights'
import type { MeasureRange } from './previewSelection'

// One discriminated ref rather than a separate measure-select and note-select
// ref: a single click can only ever anchor one mode (see
// `handlePreviewClick`'s idle-click branch), and two independent refs risk
// both firing at once. This only governs which mode a click *anchors* — it
// doesn't stop a single anchored mode's hover/second-click resolution from
// resolving cells of more than one type. Both 'note' and 'lyric' mode union
// in the other cell type's marquee hits (via `applyNoteRangeHighlights`/
// `applyLyricRangeHighlights`, which are stateless pure functions over the
// note/lyric specs, not stateful refs), the same way 'measure' mode always
// has.
//
// Selection is a click-and-click gesture, not a held-button drag: a first
// click anchors one of the modes below and eagerly highlights whatever it
// landed on; a plain `mousemove` (no button held) live-updates the
// anchor→hover marquee for mouse users; a second click resolves the range
// between the anchor and wherever that click landed and commits it (see
// `handlePreviewClick`). This is what lets the same gesture work for touch —
// a tap synthesizes a `click` with no intervening movement, so two taps are
// just two clicks with the marquee collapsing to whatever's under the second
// tap.
//
// A click on a note doesn't resolve to just that one note/chord cell right
// away — it anchors 'note' mode with `anchorId` recorded alongside the live
// `anchor`/`current` points, so the second click can either widen into a
// real marquee (a different target) or, if it lands back on that same note,
// fall back to `anchorId` rather than resolving to nothing — see
// `handlePreviewClick`'s 'note' commit branch. A click that misses every
// note/lyric click target but still lands inside a measure (a bar-line/
// gutter pixel) anchors 'measure' mode directly instead, the same as
// holding Cmd/Ctrl at the first click or landing exactly on a bar line's own
// divider or a bar number (see `previewClickHandler.ts`) — 'note' mode is
// only ever anchored by a direct hit on a note's own click target.
export type PreviewAnchorState =
  | {
      mode: 'measure'
      anchor: MeasureRange
      current: MeasureRange
      anchorId: Extract<ClickableElementId, { kind: 'measure' }>
    }
  | {
      // A bar number's own click target anchors this instead of plain
      // 'measure' mode — the bar-number-side mirror of 'part-label-system'
      // above. `anchor`/`current` stay `MeasureRange`s exactly like
      // 'measure' mode (a single-measure range at anchor time), but
      // resolution (`resolveBarNumberSystemSelection`) expands both ends
      // out to their whole *system* before selecting — every part, in
      // every system from the anchor's system through the resolved second
      // click's system — rather than stopping at the exact measure the
      // second click landed in. See
      // `bar-number-click-and-click-selects-whole-systems.feature`.
      mode: 'bar-number-system'
      anchor: MeasureRange
      current: MeasureRange
      anchorId: Extract<ClickableElementId, { kind: 'measure' }>
    }
  | {
      mode: 'note'
      anchor: AnchorPoint
      current: AnchorPoint
      /** The note/chord `ClickableElementId` the anchoring click landed
       * directly on — used to resolve the second click when it doesn't land
       * on any note's own click target, so the gesture still collapses to a
       * single cell instead of an empty selection. See this type's doc
       * comment above. */
      anchorId: Extract<ClickableElementId, { kind: 'note' }>
    }
  | {
      mode: 'part-label'
      anchor: AnchorPoint
      current: AnchorPoint
      anchorId: Extract<ClickableElementId, { kind: 'partLabel' }>
    }
  | {
      // Cmd/Ctrl-click on a part label — the label-side mirror of 'measure'
      // mode's Cmd/Ctrl gate above. Elevates 'part-label' mode's granularity
      // from "one part, one system" to "every part in every system the
      // gesture touches": a bare click resolves to the whole system the
      // clicked label sits in, and a second click further away sweeps in
      // whole additional systems as it touches their label rows (see
      // `partLabelsInMarqueeAcrossSystems`). No `anchorId` needed here,
      // unlike every other mode — this mode is deliberately unrestricted to
      // any one system and has no `ClickableElementId` concept of its own
      // (see `PLAN-clickable-element-id-selection.md`).
      mode: 'part-label-system'
      anchor: AnchorPoint
      current: AnchorPoint
    }
  | {
      // The lyric-label mirror of 'part-label' above — a click on a verse
      // row's own label (e.g. "M:v1") anchors this instead, scoped to its
      // own system the same way, but resolving only that one verse's
      // syllables rather than a whole part's notes.
      mode: 'lyric-label'
      anchor: AnchorPoint
      current: AnchorPoint
      anchorId: Extract<ClickableElementId, { kind: 'lyricLabel' }>
    }
  | {
      // No note-style single-cell fallback needed here: a lyric syllable's
      // click target is already exactly one grid column (see
      // `LyricClickTarget`), so a second click with no meaningful movement
      // naturally resolves to just that one syllable via the marquee test
      // below with zero movement — there's no "expand to the whole measure"
      // shortcut to anchor into, unlike a note click.
      mode: 'lyric'
      anchor: AnchorPoint
      current: AnchorPoint
      /** The syllable `ClickableElementId` the anchoring click landed on —
       * drives `applyLyricRangeSelection`'s same-part-and-verse check, so a
       * second click that lands directly on another syllable in the same
       * verse resolves by `noteId` order instead of pixel geometry (see
       * `previewRangeSelection.ts`). */
      anchorId: Extract<ClickableElementId, { kind: 'lyric' }>
    }
  | null
