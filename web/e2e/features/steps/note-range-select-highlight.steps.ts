import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression test for the note range-select highlight vanishing right after
 * mouseup (see `Preview.tsx`'s `applyPersistedNoteHighlights` /
 * `selectedNoteCells` prop): the highlight used to be a one-shot imperative
 * DOM toggle that got explicitly cleared on mouseup, and — even without that
 * bug — would still have been wiped by the highlighted-SVG re-render that
 * the range-select's own Monaco selection triggers a moment later.
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" and four single-beat notes in one measure, so all four note
 * click-targets render side by side in one row and stay within the viewport
 * during the range-select.
 */
const rangeTestSource = [
  '# metadata',
  'title = "note range-select test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — line 9
].join('\n')

function noteRects(page: import('@playwright/test').Page) {
  return page.locator('rect[data-variant="note-click-target-rect"]')
}

function highlightedNotes(page: import('@playwright/test').Page) {
  return page.locator('[data-tag="note"][data-note-range-selected]')
}

Given(
  'the note range-select test fixture is loaded and note click targets have rendered',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'note-range-test.jianpu',
          userFiles: { 'note-range-test.jianpu': source },
          bin: {},
          fileIds: { 'note-range-test.jianpu': 'note-range-test-id-001' },
        }),
      )
    }, rangeTestSource)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })

    // Wait for the SVG preview to render note click targets for measure 0.
    await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
      timeout: 10_000,
    })
    await expect(noteRects(page)).toHaveCount(4, { timeout: 10_000 })
  },
)

Given(
  'the editor is focused and jumped to line 9 to prime the measure round-trip',
  async ({ page, focusEditor }) => {
    // Prime the editor/worker round-trip the same way the measure
    // range-select spec does, so the highlighted-documents re-render this
    // test is guarding against is actually wired up before we range-select.
    await focusEditor()
    await page.keyboard.press('Control+g')
    await page.keyboard.type('9')
    await page.keyboard.press('Enter')
    await expect(page.locator('button.play-measure-btn')).toHaveText(
      /Measure/,
      { timeout: 5_000 },
    )
  },
)

When(
  'I click-and-click select across notes {int} to {int}',
  async ({ page }, from: number, to: number) => {
    const box0 = await stableBoundingBox(noteRects(page).nth(from))
    const box2 = await stableBoundingBox(noteRects(page).nth(to))
    if (!box0 || !box2) {
      throw new Error(
        `Could not get bounding boxes for notes ${from} and ${to}. ` +
          'Ensure the SVG preview has rendered.',
      )
    }

    const startX = box0.x + box0.width / 2
    const startY = box0.y + box0.height / 2
    const endX = box2.x + box2.width / 2
    const endY = box2.y + box2.height / 2

    // Click-and-click a range across the first three notes.
    await clickAndClickSelect(page, startX, startY, endX, endY)
  },
)

Then(
  '{int} notes are range-selected immediately after mouseup',
  async ({ page }, count: number) => {
    // Immediately after mouseup, the highlight must still be showing (this is
    // the bug: it used to be cleared the instant mouseup ran).
    await expect(highlightedNotes(page)).toHaveCount(count)
  },
)

Then('the play-measure button switches to selection mode', async ({ page }) => {
  // The repurposed play-measure button switching to "▶ Selection" confirms a
  // note range was pushed into Monaco/App state.
  await expect(page.locator('button.play-measure-btn')).toHaveText(
    /Selection/,
    { timeout: 3_000 },
  )
})

Then(
  '{int} notes are still range-selected after the highlighted-documents re-render',
  async ({ page }, count: number) => {
    // A click-and-click note-range selection pushes a Monaco multicursor selection,
    // whose cursor-change listener debounces (300 ms) into a worker
    // round-trip that swaps the plain SVG documents for highlighted ones —
    // wiping any highlight applied only as a one-shot DOM mutation. The
    // highlight must survive that swap too.
    await page.waitForTimeout(700)
    await expect(highlightedNotes(page)).toHaveCount(count)
  },
)
