import type { MouseEvent, RefObject } from 'react'
import type { PreviewAnchorState } from './previewAnchorState'
import {
  getLyricLabelAtPoint,
  getPartLabelAtPoint,
} from './previewLabelSelection'
import {
  getBarLineMeasureAtPoint,
  getBarNumberMeasureAtPoint,
  getLyricAtPoint,
  getMeasureAtPoint,
  getNoteAtPoint,
  getSectionLabelAtPoint,
} from './previewSelection'
import {
  fireCommit,
  type HandlePreviewClickArgs,
  lyricClickableElementId,
  lyricLabelClickableElementId,
  measureClickableElementId,
  noteClickableElementId,
  partLabelClickableElementId,
  resolveSelection,
} from './previewSelectionResolver'

export type { HandlePreviewClickArgs } from './previewSelectionResolver'

/** Anchors `anchorStateRef` to `newState` and immediately self-commits its
 * single-target resolution (matching a plain click's long-standing
 * instant-select behavior — see the single-click e2e specs) — but, unlike
 * the old held-button drag model, leaves `anchorStateRef` anchored rather than
 * resetting to idle: a second click can still land elsewhere and widen this
 * into a real range (see `handleCommitClick`). */
function anchorAndCommit(
  anchorStateRef: RefObject<PreviewAnchorState>,
  newState: NonNullable<PreviewAnchorState>,
  args: HandlePreviewClickArgs,
): void {
  anchorStateRef.current = newState
  // Arms the one-shot reveal suppression (see `HandlePreviewClickArgs`'s
  // `suppressNextRevealRef` doc comment) — set here, ahead of `fireCommit`,
  // so it's already armed by the time this self-commit's Monaco selection
  // round-trip debounces into `Preview.tsx`'s scroll-to-selection effect.
  args.suppressNextRevealRef.current = true
  // Flips on the pending-second-click banner/color (see
  // `HandlePreviewClickArgs`'s `onPendingSecondClickChange` doc comment)
  // ahead of the self-commit below, same ordering rationale as
  // `suppressNextRevealRef` above.
  args.onPendingSecondClickChange?.(true)
  fireCommit(resolveSelection(newState, undefined, undefined, args), args)
}

/** Resets `anchorStateRef` to idle and re-applies the highlight `anchorState`'s
 * anchoring click already committed — used both when a second click misses
 * every recognizable target and when the gesture is cancelled via Escape
 * (see `usePreviewClickSelection`). No callback fires: the anchoring click's
 * own commit already did, and nothing about that selection has changed. */
export function cancelAnchor(
  anchorStateRef: RefObject<PreviewAnchorState>,
  anchorState: NonNullable<PreviewAnchorState>,
  args: HandlePreviewClickArgs,
): void {
  resolveSelection(anchorState, undefined, undefined, args)
  anchorStateRef.current = null
  args.onPendingSecondClickChange?.(false)
}

/** Whether `(x, y)` doesn't land on anything this gesture can resolve a
 * selection from — a second click here cancels the anchored gesture rather
 * than committing a nonsensical range (see `PreviewAnchorState`'s doc comment
 * and this module's `handleCommitClick`). Mirrors the same hit-test chain
 * `handleAnchorClick` uses for a first click, since anything that would
 * anchor a *new* gesture also counts as a recognizable target for
 * *resolving* one already in progress — including the deliberate absence of
 * a measure-bounding-box fallback (see `handleAnchorClick`'s trailing
 * comment): missing every specific target cancels the gesture even if the
 * point is still inside some measure's bounding box. */
function isEmptySpace(x: number, y: number): boolean {
  if (getPartLabelAtPoint(x, y) !== undefined) return false
  if (getLyricLabelAtPoint(x, y) !== undefined) return false
  if (getBarLineMeasureAtPoint(x, y) !== undefined) return false
  if (getBarNumberMeasureAtPoint(x, y) !== undefined) return false
  if (getLyricAtPoint(x, y) !== undefined) return false
  if (getNoteAtPoint(x, y) !== undefined) return false
  return true
}

/** The first click of a click-and-click gesture: figures out what got
 * clicked (a part/lyric label, a note/chord, a lyric syllable, or plain
 * measure space — a section label is handled ahead of this, in
 * `handlePreviewClick` itself, since it's never part of the click-and-click
 * gesture below), anchors `anchorStateRef` with the mode that gesture should
 * carry through, and self-commits that single-target selection immediately
 * (see `anchorAndCommit`). `handlePreviewClick` dispatches here when
 * `anchorStateRef` is idle. */
function handleAnchorClick(
  e: MouseEvent<HTMLDivElement>,
  args: HandlePreviewClickArgs,
): void {
  const { anchorStateRef } = args
  const partLabel = getPartLabelAtPoint(e.clientX, e.clientY)
  if (partLabel !== undefined) {
    const point = { x: e.clientX, y: e.clientY }
    // Cmd/Ctrl-click on a part label elevates the selection from "this one
    // part's system" to "every part in every system touched" — see
    // `PreviewAnchorState`'s 'part-label-system' doc comment. Checked ahead of
    // the plain part-label anchor below so it takes priority.
    if (e.metaKey || e.ctrlKey) {
      anchorAndCommit(
        anchorStateRef,
        { mode: 'part-label-system', anchor: point, current: point },
        args,
      )
      e.preventDefault()
      return
    }
    anchorAndCommit(
      anchorStateRef,
      {
        mode: 'part-label',
        anchor: point,
        current: point,
        anchorId: partLabelClickableElementId(partLabel),
      },
      args,
    )
    e.preventDefault()
    return
  }
  // The lyric-side mirror of the part-label check above — a verse row's own
  // label (e.g. "M:v1"), scoped to that one verse instead of a whole part.
  const lyricLabel = getLyricLabelAtPoint(e.clientX, e.clientY)
  if (lyricLabel !== undefined) {
    const point = { x: e.clientX, y: e.clientY }
    anchorAndCommit(
      anchorStateRef,
      {
        mode: 'lyric-label',
        anchor: point,
        current: point,
        anchorId: lyricLabelClickableElementId(lyricLabel),
      },
      args,
    )
    e.preventDefault()
    return
  }
  // Grabbing a bar line's own divider always anchors a measure-range
  // selection, no Cmd/Ctrl required: the divider is a dedicated click
  // target (`Tag::BarLine`/`AbsoluteContent::BarLineClickTarget`), so
  // landing on it is an unambiguous request to select measures, unlike a
  // plain click on a note/lyric/gutter pixel (ambiguous enough to need the
  // modifier gate below).
  const barLineRange = getBarLineMeasureAtPoint(e.clientX, e.clientY)
  if (barLineRange !== undefined) {
    anchorAndCommit(
      anchorStateRef,
      {
        mode: 'measure',
        anchor: barLineRange,
        current: barLineRange,
        anchorId: measureClickableElementId(barLineRange),
      },
      args,
    )
    e.preventDefault()
    return
  }
  // Grabbing a measure's own bar number (drawn in the directive row above)
  // always anchors a selection too, no Cmd/Ctrl required — same rationale
  // as the bar-line-handle check above: landing on the bar number itself is
  // an unambiguous request to select by measure/system, unlike a click on a
  // note/lyric/gutter pixel. Anchors 'bar-number-system' rather than plain
  // 'measure' mode, though: a bar number is the click-and-click gesture's
  // system-selection entry point (see that mode's doc comment in
  // `previewAnchorState.ts`), escalating a second click anywhere into "every
  // part, every system from here through there" instead of stopping at the
  // exact measure the second click landed in.
  const barNumberRange = getBarNumberMeasureAtPoint(e.clientX, e.clientY)
  if (barNumberRange !== undefined) {
    anchorAndCommit(
      anchorStateRef,
      {
        mode: 'bar-number-system',
        anchor: barNumberRange,
        current: barNumberRange,
        anchorId: measureClickableElementId(barNumberRange),
      },
      args,
    )
    e.preventDefault()
    return
  }
  // Cmd/Ctrl-click always selects the whole measure under the pointer,
  // regardless of what structurally sits under it (note, chord, lyric,
  // bar-line, or empty gutter) — checked ahead of the lyric/note checks
  // below so it takes priority over them. Off a bar line, this is the only
  // way to reach 'measure' mode; a plain click elsewhere resolves to
  // note/chord/syllable granularity instead (see `PreviewAnchorState`'s doc
  // comment).
  if (e.metaKey || e.ctrlKey) {
    const range = getMeasureAtPoint(e.clientX, e.clientY)
    if (range !== undefined) {
      anchorAndCommit(
        anchorStateRef,
        {
          mode: 'measure',
          anchor: range,
          current: range,
          anchorId: measureClickableElementId(range),
        },
        args,
      )
      e.preventDefault()
      return
    }
  }
  // Checked before the note click-target below: a lyric syllable's own
  // click target paints on top of (and never overlaps outside of) the
  // note's wider click-target rect, so a hit here means the click landed on
  // the syllable's own rect — see `Tag::Lyric`'s doc comment and
  // `resolve_click_target_elements`'s append order.
  const lyricCell = getLyricAtPoint(e.clientX, e.clientY)
  if (lyricCell !== undefined) {
    const point = { x: e.clientX, y: e.clientY }
    anchorAndCommit(
      anchorStateRef,
      {
        mode: 'lyric',
        anchor: point,
        current: point,
        anchorId: lyricClickableElementId(lyricCell),
      },
      args,
    )
    e.preventDefault()
    return
  }
  const noteCell = getNoteAtPoint(e.clientX, e.clientY)
  if (noteCell !== undefined) {
    const point = { x: e.clientX, y: e.clientY }
    anchorAndCommit(
      anchorStateRef,
      {
        mode: 'note',
        anchor: point,
        current: point,
        anchorId: noteClickableElementId(noteCell),
      },
      args,
    )
    e.preventDefault()
    return
  }
  // Missed every note/lyric/label/bar-line/bar-number click target — even if
  // this still landed inside a measure's bounding box (e.g. the gutter
  // around a note), that's deliberately *not* treated as "select the whole
  // measure": on a mouse that gutter miss is rare, but on touch it's the
  // common case (imprecise taps around small note/lyric hit rects), which
  // made a plain tap feel like it always selected the whole measure. Bar
  // lines and bar numbers remain unconditional whole-measure targets (see
  // the checks above); everywhere else, missing every specific target is a
  // no-op rather than a measure-wide fallback.
}

/** The second click of a click-and-click gesture: resolves the range between
 * `anchorState`'s anchor and this click and commits it, returning
 * `anchorStateRef` to idle — unless this click misses every recognizable
 * target, in which case the gesture is cancelled instead, leaving the first
 * click's own self-commit untouched (see `isEmptySpace`/`cancelAnchor`).
 * `handlePreviewClick` dispatches here when `anchorStateRef` is already
 * anchored. */
function handleCommitClick(
  e: MouseEvent<HTMLDivElement>,
  anchorState: NonNullable<PreviewAnchorState>,
  args: HandlePreviewClickArgs,
): void {
  const { anchorStateRef } = args
  if (isEmptySpace(e.clientX, e.clientY)) {
    cancelAnchor(anchorStateRef, anchorState, args)
    e.preventDefault()
    return
  }
  const point = { x: e.clientX, y: e.clientY }
  // Arms the one-shot reveal suppression (see `HandlePreviewClickArgs`'s
  // `suppressNextRevealRef` doc comment and `anchorAndCommit`'s identical
  // arming above) — without this, this second click's own self-committed
  // Monaco selection round-trips back through `Preview.tsx`'s
  // scroll-to-selection effect and re-scrolls the SVG to the range this
  // click just made, fighting whatever scroll position the user is already
  // looking at.
  args.suppressNextRevealRef.current = true
  fireCommit(resolveSelection(anchorState, point, undefined, args), args)
  anchorStateRef.current = null
  args.onPendingSecondClickChange?.(false)
  e.preventDefault()
}

/**
 * The `mousedown` dispatch for `Preview`'s SVG surface, driving the click-
 * and-click range-selection gesture: idle → a first click anchors a mode and
 * self-commits its single-target resolution (`handleAnchorClick`); anchored
 * → a second click resolves and commits the range between the anchor and
 * this click, or cancels the gesture (leaving the first click's own commit
 * in place) if it misses every recognizable target (`handleCommitClick`).
 * `usePreviewClickSelection`'s `mouseover`/`mouseout` listeners live-update
 * the highlight between the two clicks for mouse users; a touch tap synthesizes
 * `mousedown`/`mouseup` with no intervening movement, so the same two
 * dispatches cover both input types.
 *
 * A section label is checked ahead of both, unconditionally — it's a jump
 * to somewhere else entirely, never one of the click-and-click gesture's own
 * targets, so it always fires `onSectionLabelClick` immediately rather than
 * anchoring or (if a gesture is already anchored) getting swallowed as that
 * gesture's second click. An anchored gesture is cancelled first (mirroring
 * Escape — see `cancelAnchor`), reverting to whatever the anchoring click
 * already committed, since the section jump replaces it rather than
 * extending it.
 *
 * Wired to `mousedown` rather than the browser's synthesized `click` event
 * deliberately: on this codebase's target platforms, a Cmd/Ctrl-held primary
 * click doesn't reliably fire `click` at all (it's the OS-level secondary-
 * click gesture), which would silently break every Cmd/Ctrl-gated mode below
 * (`'measure'` off a bare click, `'part-label-system'`) — `mousedown` has no
 * such gap. Split out of `Preview` to keep that component under its
 * line-count cap; the per-mode marquee/range resolution itself lives in
 * `previewSelectionResolver.ts` for the same reason.
 */
export function handlePreviewClick(
  e: MouseEvent<HTMLDivElement>,
  args: HandlePreviewClickArgs,
): void {
  const { anchorStateRef, onSectionLabelClick } = args
  const sectionLabel = getSectionLabelAtPoint(e.clientX, e.clientY)
  if (sectionLabel !== undefined) {
    const anchorState = anchorStateRef.current
    if (anchorState !== null) cancelAnchor(anchorStateRef, anchorState, args)
    onSectionLabelClick?.(sectionLabel)
    e.preventDefault()
    return
  }
  const anchorState = anchorStateRef.current
  if (anchorState === null) {
    handleAnchorClick(e, args)
    return
  }
  handleCommitClick(e, anchorState, args)
}
