import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" so both measures render in one system and both part labels stay
 * within the viewport. Melody carries lyrics; Harmony doesn't, so the test
 * can also assert Harmony's absence stays a no-op rather than an error.
 *
 * Measure 0: Melody "1 2" + lyrics "do re", Harmony "5 6" (2 notes)
 * Measure 1: Melody "3 4" + lyrics "mi fa", Harmony "7 1'" (2 notes)
 */
const source = [
  '# metadata',
  'title = "part label range-select lyric test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0
  'do re', // verse 0
  '[H] 5 6',
  '',
  '[M] 3 4', // measure 1
  'mi fa', // verse 0
  "[H] 7 1'",
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'part-label-range-lyric-test.jianpu',
        userFiles: { 'part-label-range-lyric-test.jianpu': source },
        bin: {},
        fileIds: {
          'part-label-range-lyric-test.jianpu':
            'part-label-range-lyric-test-id-001',
        },
      }),
    )
  }, source)
}

async function primeMeasureSpans(page: import('@playwright/test').Page) {
  await focusEditor(page)
  await page.keyboard.press('Control+g')
  await page.keyboard.type('10')
  await page.keyboard.press('Enter')
  await expect(page.locator('button.play-measure-btn')).toHaveText(/Measure/, {
    timeout: 5_000,
  })
  await expect(
    page.locator('.preview-page [data-testid="measure-highlight"]').first(),
  ).toBeVisible({ timeout: 5_000 })
}

Given('the part-label lyric-range-select fixture is loaded', async ({ page }) => {
  await loadFixture(page)
  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
  await page.waitForSelector('[data-tag="part-label"][data-part-index="1"]', {
    timeout: 10_000,
  })
  await primeMeasureSpans(page)
})

When(
  'I click-and-click select from the Melody part label to the Harmony part label, as seen in part label click selects lyrics',
  async ({ page }) => {
    const melodyLabel = page
      .locator('[data-tag="part-label"][data-part-index="0"]')
      .first()
    const harmonyLabel = page
      .locator('[data-tag="part-label"][data-part-index="1"]')
      .first()
    await expect(melodyLabel).toBeVisible({ timeout: 5_000 })
    await expect(harmonyLabel).toBeVisible({ timeout: 5_000 })

    const melodyBox = await stableBoundingBox(melodyLabel)
    const harmonyBox = await stableBoundingBox(harmonyLabel)
    if (!melodyBox || !harmonyBox) {
      throw new Error('Could not get bounding boxes for the part labels.')
    }

    await clickAndClickSelect(
      page,
      melodyBox.x + melodyBox.width / 2,
      melodyBox.y + melodyBox.height / 2,
      harmonyBox.x + harmonyBox.width / 2,
      harmonyBox.y + harmonyBox.height / 2,
    )
  },
)

Then(
  '{int} notes are range-selected in total, as seen in part label click selects lyrics',
  async ({ page }, count: number) => {
    // Melody's 4 notes + Harmony's 4 notes = 8.
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  '{int} lyrics are range-selected in total, as seen in part label click selects lyrics',
  async ({ page }, count: number) => {
    // Only Melody carries lyrics (4 syllables); Harmony has none, so the total
    // stays 4 rather than erroring or double-counting.
    await expect(
      page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
    ).toHaveCount(count)
  },
)
