import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * A measure's own bar number (drawn in its system's shared directive row,
 * above the musical rows the plain measure click target covers) should be
 * its own hoverable/Cmd-Ctrl-clickable shortcut for selecting that measure
 * — see `BarNumberClickTarget` in ARCHITECTURE.md.
 *
 * Self-contained source (not a demo file), one measure per system so the
 * first block's bar number ("1") always draws (see
 * `layout_decoration::directive_line_should_emit`).
 *
 * Measure 0 : [M] 1 2 3 4   — 4 notes
 */
const source = [
  '# metadata',
  'title = "bar number click test"',
  'max_measures_per_system = 1',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — line 9
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'bar-number-click-test.jianpu',
        userFiles: { 'bar-number-click-test.jianpu': source },
        bin: {},
        fileIds: {
          'bar-number-click-test.jianpu': 'bar-number-click-test-id-001',
        },
      }),
    )
  }, source)
}

/** Waits for measureSpans to be primed (same priming dance other
 * measure-select specs use) so the SVG has settled before hit-testing. */
async function primeMeasureSpans(page: import('@playwright/test').Page) {
  await focusEditor(page)
  await page.keyboard.press('Control+g')
  await page.keyboard.type('9')
  await page.keyboard.press('Enter')
  await expect(page.locator('button.play-measure-btn')).toHaveText(/Measure/, {
    timeout: 5_000,
  })
  await expect(
    page.locator('.preview-page [data-testid="measure-highlight"]').first(),
  ).toBeVisible({ timeout: 5_000 })
}

Given('the bar-number-click test fixture is loaded', async ({ page }) => {
  await loadFixture(page)
  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
  await page.waitForSelector('[data-tag="bar-number"]', { timeout: 10_000 })
})

function barNumberRect(page: import('@playwright/test').Page) {
  return page
    .locator(
      'g[data-tag="bar-number"] > rect[data-variant="bar-number-click-target-rect"]',
    )
    .first()
}

When('I hover the bar number for measure 0', async ({ page }) => {
  const rect = barNumberRect(page)
  await expect(rect).toBeVisible({ timeout: 5_000 })
  const box = await stableBoundingBox(rect)
  if (!box) throw new Error('Could not get bounding box for the bar number.')

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
})

Then('the bar number rect fill is highlighted', async ({ page }) => {
  const rect = barNumberRect(page)
  // The :hover rule's paint can lag the mouse-move event by a frame or two
  // in a headless run, so poll rather than reading getComputedStyle once.
  await expect
    .poll(() => rect.evaluate((el) => getComputedStyle(el).fill), {
      timeout: 3_000,
    })
    .not.toMatch(/^(none|rgba?\(0, ?0, ?0, ?0\))$/)
})

When('I Cmd\\/Ctrl-click the bar number for measure 0', async ({ page }) => {
  await primeMeasureSpans(page)

  const rect = barNumberRect(page)
  await expect(rect).toBeVisible({ timeout: 5_000 })
  const box = await stableBoundingBox(rect)
  if (!box) throw new Error('Could not get bounding box for the bar number.')

  // A Cmd/Ctrl-modified plain click (mousedown + mouseup at the same point,
  // no drag) — a bar number's own click target always selects the whole
  // measure regardless of the modifier (see
  // `previewMouseDownHandler.ts`'s unconditional bar-number check), so this
  // exercises the same path a modifier-free click would.
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.up()
  await page.keyboard.up('Control')
})

When('I plain-click the bar number for measure 0', async ({ page }) => {
  await primeMeasureSpans(page)

  const rect = barNumberRect(page)
  await expect(rect).toBeVisible({ timeout: 5_000 })
  const box = await stableBoundingBox(rect)
  if (!box) throw new Error('Could not get bounding box for the bar number.')

  // A plain click (no modifier) — the bar number's own click target always
  // selects the whole measure, no Cmd/Ctrl required (see
  // `previewMouseDownHandler.ts`'s unconditional bar-number check).
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.mouse.down()
  await page.mouse.up()
})

Then(
  '{int} notes are range-selected, as seen in bar number click selects measure',
  async ({ page }, count: number) => {
    // Measure 0 ("1 2 3 4") has exactly 4 notes.
    const highlightedNotes = page.locator(
      '[data-tag="note"][data-note-range-selected]',
    )
    await expect(highlightedNotes).toHaveCount(count)
  },
)

Then(
  'the play-measure button reads Selection, as seen in bar number click selects measure',
  async ({ page }) => {
    await expect(page.locator('button.play-measure-btn')).toHaveText(
      /Selection/,
      { timeout: 3_000 },
    )
  },
)
