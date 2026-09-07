import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * The visible bar line (measure divider) between two measures should be a
 * reliable, hoverable click target for measure-range selection: a
 * click-and-click range starting exactly on the divider pixel between
 * measure 0 and measure 1 must select whole measures, not fall into a
 * per-note marquee (see `Tag::BarLine`/`AbsoluteContent::BarLineClickTarget`,
 * consumed by `PreviewSvgRenderer.tsx`'s `groupAttrsForTag`). Grabbing the
 * divider itself always starts this gesture, with or without Cmd/Ctrl held —
 * a plain click-and-click elsewhere (off the divider) resolves to
 * note/chord/syllable granularity instead, and needs Cmd/Ctrl to reach
 * whole-measure selection (see `previewMouseDownHandler.ts`'s
 * `onMouseDown`).
 *
 * Same fixture as `measure-click-selects-notes.spec.ts`:
 * Measure 0 : [M] 1 2 3 4   — 4 notes
 * Measure 1 : [M] 5 6       — 2 notes
 * Measure 2 : [M] 7 1'      — 2 notes
 */
const barLineTestSource = [
  '# metadata',
  'title = "bar line range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0
  '',
  '[M] 5 6', // measure 1
  '',
  "[M] 7 1'", // measure 2
].join('\n')

async function loadBarLineTestFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'bar-line-range-test.jianpu',
        userFiles: { 'bar-line-range-test.jianpu': source },
        bin: {},
        fileIds: { 'bar-line-range-test.jianpu': 'bar-line-range-test-id-001' },
      }),
    )
  }, barLineTestSource)
}

/** Waits for measureSpans to be primed (same priming dance
 * `measure-click-selects-notes.spec.ts` uses) so the SVG has settled before
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

Given('the bar-line-range-select test fixture is loaded', async ({ page }) => {
  await loadBarLineTestFixture(page)
  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
})

When(
  'I hover the bar line between measure 0 and measure 1',
  async ({ page }) => {
    await page.waitForSelector('[data-tag="measure"][data-measure-index="1"]', {
      timeout: 10_000,
    })
    await primeMeasureSpans(page)

    const measure1 = page
      .locator('[data-tag="measure"][data-measure-index="1"]')
      .first()
    const box = await stableBoundingBox(measure1)
    if (!box) throw new Error('Could not get bounding box for measure 1.')

    // The bar line between measure 0 and measure 1 sits at measure 1's own
    // left edge (see `measure_column_bounds`).
    await page.mouse.move(box.x, box.y + box.height / 2)
  },
)

Then('the bar-line click-target shows a col-resize cursor', async ({ page }) => {
  const handle = page
    .locator('rect[data-variant="bar-line-click-target-rect"]')
    .first()
  await expect(handle).toHaveCSS('cursor', 'col-resize')
})

When(
  "I Cmd\\/Ctrl-click-and-click from the bar line before measure 1 into measure 2's interior",
  async ({ page }) => {
    await page.waitForSelector('[data-tag="measure"][data-measure-index="2"]', {
      timeout: 10_000,
    })
    await primeMeasureSpans(page)

    const measure1 = page
      .locator('[data-tag="measure"][data-measure-index="1"]')
      .first()
    const measure2 = page
      .locator('[data-tag="measure"][data-measure-index="2"]')
      .first()
    const box1 = await stableBoundingBox(measure1)
    const box2 = await stableBoundingBox(measure2)
    if (!box1 || !box2) {
      throw new Error('Could not get bounding boxes for measures 1 and 2.')
    }

    // Click-and-click starting exactly on the bar line between measure 0 and
    // measure 1 (measure 1's own left edge), then into measure 2's interior.
    // Held under Cmd/Ctrl (see this file's top-of-file comment).
    await page.keyboard.down('Control')
    await clickAndClickSelect(
      page,
      box1.x,
      box1.y + box1.height / 2,
      box2.x + box2.width / 2,
      box2.y + box2.height / 2,
      8,
    )
    await page.keyboard.up('Control')
  },
)

When(
  "I plain click-and-click from the bar line before measure 1 into measure 2's interior",
  async ({ page }) => {
    await page.waitForSelector('[data-tag="measure"][data-measure-index="2"]', {
      timeout: 10_000,
    })
    await primeMeasureSpans(page)

    const measure1 = page
      .locator('[data-tag="measure"][data-measure-index="1"]')
      .first()
    const measure2 = page
      .locator('[data-tag="measure"][data-measure-index="2"]')
      .first()
    const box1 = await stableBoundingBox(measure1)
    const box2 = await stableBoundingBox(measure2)
    if (!box1 || !box2) {
      throw new Error('Could not get bounding boxes for measures 1 and 2.')
    }

    // Same gesture as the Cmd/Ctrl scenario above, but with no modifier key
    // held — grabbing the bar line's own divider is an unambiguous request to
    // select measures on its own, regardless of modifier keys.
    await clickAndClickSelect(
      page,
      box1.x,
      box1.y + box1.height / 2,
      box2.x + box2.width / 2,
      box2.y + box2.height / 2,
      8,
    )
  },
)

Then(
  '{int} notes are range-selected, as seen in bar line click selects measures',
  async ({ page }, count: number) => {
    // Measures 1-2 have 2 + 2 = 4 notes in total — the full measure range, not
    // a partial note marquee.
    const highlightedNotes = page.locator(
      '[data-tag="note"][data-note-range-selected]',
    )
    await expect(highlightedNotes).toHaveCount(count)
  },
)

Then('the play-measure button reads Selection', async ({ page }) => {
  await expect(page.locator('button.play-measure-btn')).toHaveText(
    /Selection/,
    { timeout: 3_000 },
  )
})
