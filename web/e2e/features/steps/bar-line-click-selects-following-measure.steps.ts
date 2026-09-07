import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Cmd/Ctrl-clicking anywhere on a bar line's own click target
 * (`[data-tag="bar-line"]`, see `Tag::BarLine`/
 * `AbsoluteContent::BarLineClickTarget`) must resolve to the measure *after*
 * that line, never the one before — the click target is a fixed-width rect
 * padded wider than the visible stroke specifically so a real mouse doesn't
 * have to land on the exact boundary pixel, and that padding must not flip
 * which measure the click resolves to, since resolution reads the target's
 * own `data-measure-index-next`/`data-measure-index-prev` identity rather
 * than pixel geometry (see `previewSelection.ts`'s
 * `getBarLineMeasureAtPoint`). The Cmd/Ctrl modifier is kept on these tests
 * for parity with the general "select this measure" gesture, though it's no
 * longer load-bearing for the fallback case: a click that misses the
 * bar-line's own padded click target now resolves to the whole measure
 * regardless of the modifier (see `previewClickHandler.ts`'s
 * `handleAnchorClick`).
 *
 * The sole exception is a system's *last* bar line — its closing line, with
 * no following measure on the same row — which resolves to the measure
 * *before* it instead, since there's nothing after it to select.
 *
 * `max_measures_per_system = 2` forces measures 0-1 onto the first system
 * row and measure 2 onto a second row, so the bar line closing measure 1 is
 * a genuine "last bar line of a system" distinct from the score's own final
 * bar line.
 *
 * Measure 0 : [M] 1 2 3 4   — 4 notes
 * Measure 1 : [M] 5 6       — 2 notes
 * Measure 2 : [M] 7 1'      — 2 notes
 */
const source = [
  '# metadata',
  'title = "bar line click test"',
  'max_measures_per_system = 2',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0
  '',
  '[M] 5 6', // measure 1 — last measure of system 0
  '',
  "[M] 7 1'", // measure 2 — first measure of system 1
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'bar-line-click-test.jianpu',
        userFiles: { 'bar-line-click-test.jianpu': source },
        bin: {},
        fileIds: { 'bar-line-click-test.jianpu': 'bar-line-click-test-id-001' },
      }),
    )
  }, source)
}

/** Waits for measureSpans to be primed (same priming dance the other
 * bar-line/measure-click specs use) so the SVG has settled before
 * hit-testing. */
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

Given('the bar-line-click test fixture is loaded', async ({ page }) => {
  await loadFixture(page)
  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
  await page.waitForSelector('[data-tag="measure"][data-measure-index="1"]', {
    timeout: 10_000,
  })
  await primeMeasureSpans(page)
})

When(
  "I Cmd\\/Ctrl-click a few pixels left of measure 1's left edge",
  async ({ page }) => {
    await page.waitForSelector('[data-tag="measure"][data-measure-index="1"]', {
      timeout: 10_000,
    })

    const measure1 = page
      .locator('[data-tag="measure"][data-measure-index="1"]')
      .first()
    const box1 = await stableBoundingBox(measure1)
    if (!box1) throw new Error('Could not get bounding box for measure 1.')

    // Click a couple pixels to the *left* of measure 1's true left edge —
    // still inside the bar line's wide invisible hit target, but on the
    // measure-0 side of the true boundary pixel. Held under Cmd/Ctrl (see
    // this file's top-of-file comment).
    await page.mouse.move(box1.x - 2, box1.y + box1.height / 2)
    await page.keyboard.down('Control')
    await page.mouse.down()
    await page.mouse.up()
    await page.keyboard.up('Control')
  },
)

When(
  "I Cmd\\/Ctrl-click measure 1's right edge, which is the last bar line of its system",
  async ({ page }) => {
    await page.waitForSelector('[data-tag="measure"][data-measure-index="2"]', {
      timeout: 10_000,
    })

    const measure1 = page
      .locator('[data-tag="measure"][data-measure-index="1"]')
      .first()
    const box1 = await stableBoundingBox(measure1)
    if (!box1) throw new Error('Could not get bounding box for measure 1.')

    // The bar line closing system 0 sits at measure 1's own right edge. Held
    // under Cmd/Ctrl (see this file's top-of-file comment).
    await page.mouse.move(box1.x + box1.width, box1.y + box1.height / 2)
    await page.keyboard.down('Control')
    await page.mouse.down()
    await page.mouse.up()
    await page.keyboard.up('Control')
  },
)

Then('{int} notes are range-selected', async ({ page }, count: number) => {
  // Measure 1 ("5 6") has exactly 2 notes; measure 0 ("1 2 3 4") has 4 — a
  // fall-back to measure 0 would show 4 notes selected instead.
  const highlightedNotes = page.locator(
    '[data-tag="note"][data-note-range-selected]',
  )
  await expect(highlightedNotes).toHaveCount(count)
})

Then('note ids {int} and {int} are range-selected', async ({ page }, a, b) => {
  // Measure 1 ("5 6") has exactly 2 notes; measure 2 ("7 1'") also has 2, but
  // with different note ids — assert on those ids to distinguish the two.
  for (const noteId of [a, b]) {
    await expect(
      page.locator(
        `[data-tag="note"][data-note-range-selected][data-note-id="${noteId}"]`,
      ),
    ).toHaveCount(1)
  }
})
