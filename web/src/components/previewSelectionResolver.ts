import type { RefObject } from 'react'
import type { LyricSpan, NoteSpan } from '../types'
import type { ClickableElementId } from './clickableElementId'
import type { PreviewAnchorState } from './previewAnchorState'
import type { LyricCell, NoteCell } from './previewSelection'

export {
  lyricClickableElementId,
  lyricLabelClickableElementId,
  measureClickableElementId,
  noteClickableElementId,
  partLabelClickableElementId,
} from './previewClickableElementIdBuilders'

import {
  resolveLyricLabelSelection,
  resolvePartLabelSelection,
  resolvePartLabelSystemSelection,
} from './previewSelectionResolveLabelModes'
import {
  resolveBarNumberSystemSelection,
  resolveLyricSelection,
  resolveMeasureSelection,
  resolveNoteSelection,
} from './previewSelectionResolveModes'

/** Shared by `previewClickHandler.ts`'s idle/anchored dispatch and
 * `usePreviewClickSelection`'s Escape handler — everything a click-and-click
 * gesture needs to resolve, highlight, and commit a selection. */
export interface HandlePreviewClickArgs {
  anchorStateRef: RefObject<PreviewAnchorState>
  /** Armed by a gesture's anchoring (first) click — see
   * `previewClickHandler.ts`'s `anchorAndCommit` — and consumed once by
   * `Preview.tsx`'s scroll-to-selection effect, so that self-commit's own
   * debounced Monaco-selection round-trip doesn't auto-scroll the preview
   * out from under a user still moving toward a second click (e.g. onto
   * another page) to widen the gesture into a range. Deliberately a
   * one-shot flag rather than a persistent "is a gesture anchored" check:
   * a click-and-click gesture's anchor stays live indefinitely (until a
   * second click or Escape), so gating on that directly would silently
   * suppress every *later*, unrelated reveal too (e.g. a keyboard
   * navigation) for as long as an old anchor from a single, never-followed-up
   * click happens to still be sitting there. */
  suppressNextRevealRef: RefObject<boolean>
  previewPagesRef: RefObject<HTMLDivElement | null>
  /** Notified `true` the instant a click-and-click gesture anchors (a first
   * click sets `anchorStateRef`) and `false` the instant it resolves — a
   * second click's commit or a cancellation (empty-space click, Escape, a
   * section-label jump interrupting it; see `cancelAnchor`). Drives both the
   * "click again to select a range" banner and the pending-selection
   * highlight color (see `data-pending-selection` in `index.css`) —
   * `usePreviewClickSelection` owns the backing state and passes its setter
   * through here so `previewClickHandler.ts`'s anchor/commit/cancel paths
   * can flip it without needing React state of their own. */
  onPendingSecondClickChange: ((pending: boolean) => void) | undefined
  noteSpans: NoteSpan[]
  lyricSpans: LyricSpan[]
  onSectionLabelClick: ((label: string) => void) | undefined
  onNoteRangeSelect: ((selectedCells: NoteCell[]) => void) | undefined
  onLyricRangeSelect: ((selectedCells: LyricCell[]) => void) | undefined
  onMeasureRangeSelect:
    | ((noteCells: NoteCell[], lyricCells: LyricCell[]) => void)
    | undefined
}

export interface ResolvedSelection {
  noteCells: NoteCell[]
  lyricCells: LyricCell[]
}

/**
 * The per-mode marquee/range resolution shared by every point
 * `previewClickHandler.ts` needs it: the first click's immediate
 * self-commit, a second click's final commit, and reverting a cancelled
 * second click back to what the first click already committed. `point` is
 * the screen point (or, for 'measure' mode, resolved into a measure range
 * internally) the gesture should resolve against as its "current" side of
 * the anchor→current marquee — pass `undefined` to resolve against the
 * anchor itself (i.e. zero movement), which is exactly what the first
 * click's own self-commit and a cancelled second click's revert both need.
 *
 * `currentIdHint`, when given, is used as `current`'s `ClickableElementId`
 * directly instead of re-deriving one from `point` via
 * `anyClickableElementIdAtPoint`/`getMeasureAtPoint` — `usePreviewClickSelection`'s
 * `mouseover`/`mouseout` hover listener already resolves the hovered
 * element's own ID off its `dataset` (no pixel hit-test needed) and passes
 * it straight through here, unifying the live hover preview onto the exact
 * same resolution the commit path uses. Every other caller (the first
 * click's self-commit, a second click's final commit, `cancelAnchor`)
 * passes `undefined`, keeping their point-based resolution unchanged.
 *
 * Applies the resulting highlight to the DOM as a side effect and returns
 * the cells it resolved, but never fires a callback or touches
 * `anchorStateRef` — callers own that.
 *
 * Dispatches to one per-mode resolver — `resolveMeasureSelection`/
 * `resolveBarNumberSystemSelection`/`resolveNoteSelection`/
 * `resolveLyricSelection` in `previewSelectionResolveModes.ts`,
 * `resolvePartLabelSelection`/`resolvePartLabelSystemSelection`/
 * `resolveLyricLabelSelection` in `previewSelectionResolveLabelModes.ts` —
 * split out once each mode's own branch grew too long to keep inline here.
 */
export function resolveSelection(
  anchorState: NonNullable<PreviewAnchorState>,
  point: { x: number; y: number } | undefined,
  currentIdHint: ClickableElementId | undefined,
  { previewPagesRef, noteSpans, lyricSpans }: HandlePreviewClickArgs,
): ResolvedSelection {
  const args = {
    container: previewPagesRef.current,
    point,
    currentIdHint,
    noteSpans,
    lyricSpans,
  }

  switch (anchorState.mode) {
    case 'measure':
      return resolveMeasureSelection(anchorState, args)
    case 'bar-number-system':
      return resolveBarNumberSystemSelection(anchorState, args)
    case 'note':
      return resolveNoteSelection(anchorState, args)
    case 'lyric':
      return resolveLyricSelection(anchorState, args)
    case 'part-label':
      return resolvePartLabelSelection(anchorState, args)
    case 'part-label-system':
      return resolvePartLabelSystemSelection(anchorState, args)
    case 'lyric-label':
      return resolveLyricLabelSelection(anchorState, args)
  }
}

/** Fires whichever of `onNoteRangeSelect`/`onLyricRangeSelect`/
 * `onMeasureRangeSelect` is wired up for a resolved selection — shared by
 * the first click's immediate self-commit and a second click's final
 * commit. */
export function fireCommit(
  { noteCells, lyricCells }: ResolvedSelection,
  args: HandlePreviewClickArgs,
): void {
  if (args.onMeasureRangeSelect) {
    args.onMeasureRangeSelect(noteCells, lyricCells)
  } else {
    args.onNoteRangeSelect?.(noteCells)
    args.onLyricRangeSelect?.(lyricCells)
  }
}
