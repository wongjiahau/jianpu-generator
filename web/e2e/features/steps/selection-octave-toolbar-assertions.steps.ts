import { expect } from '@playwright/test'
import { Then, When } from './fixtures'

// "the editor source contains {string}", "the stored source contains
// {string}", and "the editor source still contains {string}" are all
// already defined in `shift-part-octave-toolbar.steps.ts` with identical
// semantics (both fixtures use the same `jianpu:files:v1` storage shape) —
// playwright-bdd registers steps globally, so redefining them here would
// conflict rather than shadow.

When(
  'I click the {string} editor toolbar button',
  async ({ page }, label: string) => {
    await page.click(`.editor-toolbar-button[aria-label="${label}"]`)
  },
)

Then(
  'the {string} editor toolbar button is disabled',
  async ({ page }, label: string) => {
    await expect(
      page.locator(`.editor-toolbar-button[aria-label="${label}"]`),
    ).toBeDisabled()
  },
)

/** Reads the `(sourcePartIndex, noteId)` identity of every note currently
 * carrying the SVG preview's persisted selection highlight (driven off
 * `selectedNoteCells`, see `Preview.tsx`'s `applyPersistedNoteHighlights`).
 *
 * Deliberately scoped to the `[data-tag="note"]` group that encloses the
 * *click-target* rect, not every `[data-tag="note"][data-note-drag-selected]`
 * element: each note also has a second, sibling `[data-tag="note"]` group for
 * its (pointer-events: none) playback-cursor rect (see
 * `applyPersistedNoteHighlights`'s doc comment in `previewDragHighlights.ts`),
 * and `applyPersistedHighlights` only ever visits/clears the click-target
 * one — the playback-cursor sibling can be left carrying a stale flag from
 * an earlier selection indefinitely. That's harmless in the real app (the
 * CSS rule painting the highlight only matches the click-target rect, so the
 * stale flag on the other sibling is never actually rendered), but a query
 * that doesn't scope to the click-target rect the same way the CSS does
 * would misread that inert leftover as a real highlighted note. */
async function highlightedNoteCellKeys(page: import('@playwright/test').Page) {
  return page.evaluate(() =>
    Array.from(
      document.querySelectorAll(
        '[data-tag="note"][data-note-drag-selected] rect[data-variant="note-click-target-rect"]',
      ),
    )
      .map((rect) => rect.closest('[data-tag="note"]') as HTMLElement)
      .map((group) => `${group.dataset.partIndex}:${group.dataset.noteId}`)
      .sort(),
  )
}

// Module-level, not scenario state on `page` — read back within the same
// scenario's own sequential steps only, mirroring the pattern other step
// files in this suite use for a "remember X, then assert it later" flow.
let rememberedHighlightedNoteCellKeys: string[] = []

When(
  'I remember which notes are highlighted in the SVG preview',
  async ({ page }) => {
    await expect
      .poll(async () => (await highlightedNoteCellKeys(page)).length, {
        timeout: 10_000,
      })
      .toBeGreaterThan(0)
    rememberedHighlightedNoteCellKeys = await highlightedNoteCellKeys(page)
  },
)

// Regression coverage for the octave-shift toolbar action silently
// re-deriving this highlight from stale `noteSpans` (see
// `useAppSelectionAndNavigation.ts`'s `handleShiftSelectionOctave`): the
// note *identity* set — not just the count — must survive the shift, since
// a byte-offset drift between an earlier-in-source shifted note and a later
// one can swap which notes an overlap test matches while keeping the count
// the same.
Then(
  'the same notes are still highlighted in the SVG preview',
  async ({ page }) => {
    await expect
      .poll(() => highlightedNoteCellKeys(page))
      .toEqual(rememberedHighlightedNoteCellKeys)
  },
)

/** Asserts the editor's current selection is still exactly `count` Monaco
 * `Selection` range(s) — i.e. a mouse drag's one contiguous block hasn't
 * been replaced by a disjoint multicursor selection (one per shifted note),
 * which would silently narrow what the user selected even though it still
 * covers every shifted note. */
Then(
  'the editor still has exactly {int} selection range',
  async ({ page }, count: number) => {
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            window.monaco?.editor.getEditors()[0]?.getSelections()?.length ?? 0,
        ),
      )
      .toBe(count)
  },
)
