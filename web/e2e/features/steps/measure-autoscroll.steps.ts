import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Generates a source with `count` single-measure lines so the rendered SVG
 * preview overflows `.preview-pages` vertically, making auto-scroll
 * observable.
 *
 * Each measure is its own `[M] 1 2 3 4` line (1-based Monaco line number
 * `11 + i * 2`, since measures are separated by a blank line).
 */
function buildLongSource(count: number): string {
  const lines = [
    '# metadata',
    'title = "autoscroll test"',
    '',
    '# parts',
    'Melody [M] = notes',
    '',
    '# score',
  ]
  for (let i = 0; i < count; i++) {
    lines.push('[M] 1 2 3 4')
    lines.push('')
  }
  return lines.join('\n')
}

const measureCount = 60
let scrollTopBefore: number

/**
 * Regression/feature test: moving the caret to a measure that is scrolled
 * out of view must auto-scroll the SVG preview so the highlighted measure
 * becomes visible (Preview.tsx calls `scrollIntoView({ block: 'center' })`
 * whenever `highlightedDocuments` changes).
 */
Given('a 60-measure autoscroll test fixture is loaded', async ({ page }) => {
  const source = buildLongSource(measureCount)

  await page.addInitScript((src) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'autoscroll-test.jianpu',
        userFiles: { 'autoscroll-test.jianpu': src },
        bin: {},
        fileIds: { 'autoscroll-test.jianpu': crypto.randomUUID() },
      }),
    )
  }, source)

  await page.goto('/')
  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })

  const lastMeasureIndex = measureCount - 1
  await page.waitForSelector(
    `[data-tag="measure"][data-measure-index="${lastMeasureIndex}"]`,
    { timeout: 15_000 },
  )
})

When("I jump the caret to the first measure's line", async ({ page }) => {
  // Place the caret on the first measure (line 8: "[M] 1 2 3 4") and confirm
  // the preview has not scrolled away from the top yet.
  await focusEditor(page)
  await page.keyboard.press('Control+g')
  await page.keyboard.type('8')
  await page.keyboard.press('Enter')
  await expect(page.getByTestId('play-measure-button')).toContainText(
    'Measure 1',
    { timeout: 5_000 },
  )
  await expect(
    page.locator('.preview-page [data-testid="measure-highlight"]').first(),
  ).toBeVisible({ timeout: 5_000 })
})

Then('the preview has not scrolled away from the top', async ({ page }) => {
  const previewPages = page.locator('.preview-pages')
  scrollTopBefore = await previewPages.evaluate((el) => el.scrollTop)
})

When("I jump the caret to the last measure's line", async ({ page }) => {
  const lastMeasureIndex = measureCount - 1
  // Jump the caret to the last measure's line, which should be scrolled far
  // out of view given 60 measures.
  const lastMeasureLine = 8 + lastMeasureIndex * 2
  await page.keyboard.press('Control+g')
  await page.keyboard.type(String(lastMeasureLine))
  await page.keyboard.press('Enter')
  await expect(page.getByTestId('play-measure-button')).toContainText(
    `Measure ${measureCount}`,
    { timeout: 5_000 },
  )
})

Then(
  "the preview scrolls so the last measure's highlight is visible in the viewport",
  async ({ page }) => {
    const lastMeasureIndex = measureCount - 1
    const previewPages = page.locator('.preview-pages')

    // Wait for the rAF-driven scrollIntoView to run.
    await expect
      .poll(async () => previewPages.evaluate((el) => el.scrollTop), {
        timeout: 5_000,
      })
      .not.toBe(scrollTopBefore)

    // The highlight rect is drawn on a separate overlay SVG document (not
    // nested inside the plain document's `[data-tag="measure"]` group), so
    // confirm it's the last measure's highlight by matching its position
    // against the last measure group's bounding box, then assert it is
    // within the preview viewport.
    const lastMeasureGroup = page
      .locator(`[data-tag="measure"][data-measure-index="${lastMeasureIndex}"]`)
      .first()
    const highlightRect = page
      .locator('.preview-page [data-testid="measure-highlight"]')
      .first()

    await expect(highlightRect).toBeVisible({ timeout: 5_000 })

    const groupBox = await stableBoundingBox(lastMeasureGroup)
    const highlightBox = await stableBoundingBox(highlightRect)
    if (!groupBox || !highlightBox) {
      throw new Error('Could not get bounding boxes for the last measure.')
    }
    expect(Math.abs(groupBox.x - highlightBox.x)).toBeLessThan(5)
    expect(Math.abs(groupBox.y - highlightBox.y)).toBeLessThan(5)

    await expect(highlightRect).toBeInViewport({ ratio: 0.5 })
  },
)
