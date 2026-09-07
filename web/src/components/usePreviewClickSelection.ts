import type { RefObject } from 'react'
import { useEffect, useRef, useState } from 'react'
import type { LyricSpan, NoteSpan } from '../types'
import type { ClickableElementId } from './clickableElementId'
import { clickableElementIdFromElement } from './clickableElementId'
import { cancelAnchor } from './previewClickHandler'
import type { PreviewAnchorState } from './previewAnchorState'
import {
  getMeasureAtPoint,
  type LyricCell,
  type NoteCell,
} from './previewSelection'
import {
  type HandlePreviewClickArgs,
  resolveSelection,
} from './previewSelectionResolver'

export type { PreviewAnchorState } from './previewAnchorState'

/** The `ClickableElementId` for a `mouseover`'s own target, if any — the
 * delegated-event counterpart of `previewSelection.ts`'s point-based
 * hit-tests, used by `usePreviewClickSelection`'s hover listener below.
 * `target` need not be the `[data-tag]` group itself (unlike
 * `clickableElementIdFromElement`'s own contract): this walks up to the
 * nearest one via `closest`, mirroring how every point-based hit-test walks
 * up from `elementFromPoint`'s raw result. */
function hoveredElementId(
  target: EventTarget | null,
): ClickableElementId | undefined {
  if (!(target instanceof Element)) return undefined
  const group = target.closest('[data-tag]')
  return group ? clickableElementIdFromElement(group) : undefined
}

/** Owns the note/measure/part-label click-and-click selection gesture for
 * `Preview`: a first click (handled by `Preview` itself via
 * `handlePreviewClick`, which writes the anchored mode into the returned
 * `anchorStateRef`) anchors one of `PreviewAnchorState`'s modes, and the
 * `mouseover`/`mouseout` listeners registered here live-update the hover
 * preview between the anchor and the pointer for mouse users (a no-op for
 * touch, which has no hover) until a second click — also routed through
 * `handlePreviewClick` — resolves and commits it.
 *
 * Delegates through `resolveSelection` (`previewSelectionResolver.ts`) on
 * every tick, the same resolver the commit path already uses, rather than a
 * separate hand-rolled per-mode marquee/index computation: `mouseover`'s
 * `event.target.closest('[data-tag]')` identifies the hovered element
 * directly off its own `data-*` attributes (`clickableElementIdFromElement`),
 * with no `elementFromPoint`/`elementsFromPoint` pixel scan — except for
 * 'measure' and 'bar-number-system' modes, the documented exceptions (see
 * `PLAN-clickable-element-id-selection.md`'s hover-migration entry and
 * `clickableElementIdFromElement`'s own doc comment for why a note/lyric's
 * click-target rect, being a DOM *sibling* of the measure/bar-line group
 * rather than its ancestor, can't be reached by `closest()` alone).
 * `mouseout` only resolves on a genuine boundary-leave (the pointer leaving
 * the container entirely) — every element-to-element transition inside it
 * is already handled by the corresponding `mouseover`.
 *
 * A document-level `keydown` listener cancels the anchored gesture back to
 * idle on Escape, the click-click model's equivalent of releasing a held
 * button to abort. Also returns `suppressNextRevealRef` — see
 * `HandlePreviewClickArgs`'s doc comment — for `Preview.tsx`'s
 * scroll-to-selection effect to consume. Split out of `Preview` to keep
 * that component under its line-count cap. */
export function usePreviewClickSelection(
  previewPagesRef: RefObject<HTMLDivElement | null>,
  noteSpans: NoteSpan[],
  onNoteRangeSelect: ((selectedCells: NoteCell[]) => void) | undefined,
  onLyricRangeSelect?: (selectedCells: LyricCell[]) => void,
  lyricSpans: LyricSpan[] = [],
  // Fired instead of `onNoteRangeSelect`/`onLyricRangeSelect` for a
  // measure/bar-line click, which resolves both note cells and lyric cells
  // at once — see `useAppController`'s `handleMeasureRangeSelect` for why
  // those two can't just be called back-to-back (each independently pushes
  // its own Monaco selection, and the second call's push clobbers the
  // first's).
  onMeasureRangeSelect?: (
    noteCells: NoteCell[],
    lyricCells: LyricCell[],
  ) => void,
) {
  // The mouseover/mouseout/keydown handlers below live in a
  // `useEffect(() => {...}, [previewPagesRef])`, so they'd otherwise close
  // over the `noteSpans`/`onNoteRangeSelect` from that first render — refs
  // keep them reading the latest value.
  const noteSpansRef = useRef(noteSpans)
  noteSpansRef.current = noteSpans
  const lyricSpansRef = useRef(lyricSpans)
  lyricSpansRef.current = lyricSpans
  const onNoteRangeSelectRef = useRef(onNoteRangeSelect)
  onNoteRangeSelectRef.current = onNoteRangeSelect
  const onLyricRangeSelectRef = useRef(onLyricRangeSelect)
  onLyricRangeSelectRef.current = onLyricRangeSelect
  const onMeasureRangeSelectRef = useRef(onMeasureRangeSelect)
  onMeasureRangeSelectRef.current = onMeasureRangeSelect

  const anchorStateRef = useRef<PreviewAnchorState>(null)
  // See `HandlePreviewClickArgs`'s doc comment — owned here (alongside
  // `anchorStateRef`) so `Preview.tsx`'s scroll-to-selection effect can
  // consume it, and passed through to every `HandlePreviewClickArgs` built
  // below.
  const suppressNextRevealRef = useRef(false)

  // Whether a click-and-click gesture is anchored and waiting on its second
  // click — real React state (unlike `anchorStateRef` itself) since
  // `Preview.tsx` needs a render to show/hide the "click again to select a
  // range" banner. `previewClickHandler.ts`'s anchor/commit/cancel paths
  // flip this via `onPendingSecondClickChange` (see `HandlePreviewClickArgs`)
  // rather than `Preview.tsx` deriving it from `anchorStateRef` itself, since a
  // ref mutation alone triggers no re-render.
  const [pendingSecondClick, setPendingSecondClick] = useState(false)

  // Mirrors `pendingSecondClick` onto the preview container as a `data-*`
  // attribute so `index.css` can paint the anchor's own highlight in a
  // distinct "pending" color (`[data-pending-selection] ...`) instead of the
  // normal committed-selection color — every mode's resolver ultimately
  // paints through `applyPersistedNoteHighlights`/`applyPersistedLyricHighlights`
  // (see `previewSelectionResolveModes.ts`/`previewSelectionResolveLabelModes.ts`),
  // so a single ancestor attribute here covers all of them without touching
  // each resolver individually.
  useEffect(() => {
    const container = previewPagesRef.current
    if (!container) return
    if (pendingSecondClick) {
      container.dataset.pendingSelection = ''
    } else {
      delete container.dataset.pendingSelection
    }
  }, [pendingSecondClick, previewPagesRef])

  useEffect(() => {
    const container = previewPagesRef.current
    if (!container) return

    const argsForHover = (): HandlePreviewClickArgs => ({
      anchorStateRef,
      suppressNextRevealRef,
      previewPagesRef,
      onPendingSecondClickChange: setPendingSecondClick,
      noteSpans: noteSpansRef.current,
      lyricSpans: lyricSpansRef.current,
      onSectionLabelClick: undefined,
      onNoteRangeSelect: onNoteRangeSelectRef.current,
      onLyricRangeSelect: onLyricRangeSelectRef.current,
      onMeasureRangeSelect: onMeasureRangeSelectRef.current,
    })

    const handleMouseOver = (e: MouseEvent) => {
      const anchorState = anchorStateRef.current
      if (!anchorState) return
      const point = { x: e.clientX, y: e.clientY }

      if (
        anchorState.mode === 'measure' ||
        anchorState.mode === 'bar-number-system'
      ) {
        // The one exception to pure element delegation (see this hook's own
        // doc comment) — keeps the point-based `getMeasureAtPoint` lookup
        // `resolveSelection` itself would otherwise fall back to anyway, so
        // `anchorState.current` (its own miss-fallback) stays fresh across
        // hover ticks the same way the old `mousemove` listener kept it.
        // 'bar-number-system' shares this: it resolves off the same
        // point→measure lookup before expanding to a system range, so it
        // needs the identical hover-side priming.
        const range = getMeasureAtPoint(e.clientX, e.clientY)
        if (range !== undefined) anchorState.current = range
        resolveSelection(anchorState, point, undefined, argsForHover())
        return
      }

      anchorState.current = point
      resolveSelection(
        anchorState,
        point,
        hoveredElementId(e.target),
        argsForHover(),
      )
    }

    const handleMouseOut = (e: MouseEvent) => {
      const anchorState = anchorStateRef.current
      if (!anchorState) return
      // Every element-to-element transition inside the container is already
      // handled by `handleMouseOver` above — only a genuine boundary-leave
      // (the pointer leaving the container entirely) needs to resolve here.
      if (
        e.relatedTarget instanceof Node &&
        container.contains(e.relatedTarget)
      )
        return
      const point = { x: e.clientX, y: e.clientY }
      resolveSelection(anchorState, point, undefined, argsForHover())
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      const anchorState = anchorStateRef.current
      if (!anchorState) return
      // Stops the event here, ahead of Monaco's own keybinding service: with
      // focus still sitting in the editor (see the capture-phase comment
      // above), an unstopped Escape falls through to Monaco's default
      // "collapse selection to the cursor" command, which fires its own
      // `onDidChangeCursorSelection` and empties `selectedNoteCells`/
      // `selectedLyricCells` right back out from under the revert this
      // handler is about to apply (see `cancelAnchor`) — even though that
      // revert itself succeeds, the very next render's declarative
      // highlight effect (`Preview.tsx`) then re-applies the now-empty
      // selection over it.
      e.preventDefault()
      e.stopPropagation()
      cancelAnchor(anchorStateRef, anchorState, argsForHover())
    }

    container.addEventListener('mouseover', handleMouseOver)
    container.addEventListener('mouseout', handleMouseOut)
    // Capture phase, not bubble: a preview click never blurs Monaco (its
    // `mousedown` handler calls `preventDefault()` — see
    // `previewClickHandler.ts` — which suppresses the browser's default
    // focus-shift-on-mousedown too), so the editor keeps focus through an
    // entire click-and-click gesture. Monaco's own keybinding service
    // intercepts Escape on its DOM node and stops it from bubbling, which
    // would otherwise mean this listener never sees it. Capture fires
    // top-down before that, so it sees Escape regardless of where in the
    // page focus sits.
    document.addEventListener('keydown', handleKeyDown, true)
    return () => {
      container.removeEventListener('mouseover', handleMouseOver)
      container.removeEventListener('mouseout', handleMouseOut)
      document.removeEventListener('keydown', handleKeyDown, true)
    }
  }, [previewPagesRef])

  return {
    anchorStateRef,
    suppressNextRevealRef,
    pendingSecondClick,
    setPendingSecondClick,
  }
}
