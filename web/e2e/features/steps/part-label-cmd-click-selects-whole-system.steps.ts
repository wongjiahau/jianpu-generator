import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Same fixture as `part-label-range-select-system-boundary.spec.ts`:
 * `max_measures_per_system = 1` forces each measure onto its own system, so
 * Melody's and Harmony's labels repeat twice, stacked vertically:
 *
 *   System 0 (measure 0): Melody "1 2", Harmony "5 6"
 *   System 1 (measure 1): Melody "3 4", Harmony "7 1'"
 */
const source = [
  '# metadata',
  'title = "part label cmd click test"',
  'max_measures_per_system = 1',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0
  '[H] 5 6',
  '',
  '[M] 3 4', // measure 1
  "[H] 7 1'",
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'part-label-cmd-click-test.jianpu',
        userFiles: { 'part-label-cmd-click-test.jianpu': source },
        bin: {},
        fileIds: {
          'part-label-cmd-click-test.jianpu':
            'part-label-cmd-click-test-id-001',
        },
      }),
    )
  }, source)
}

/** Waits for measureSpans to be primed (same priming dance the measure-select
 * specs use) so the SVG has settled before hit-testing. */
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

function partLabel(
  page: import('@playwright/test').Page,
  partIndex: number,
  measureIndexStart: number,
) {
  return page.locator(
    `[data-tag="part-label"][data-part-index="${partIndex}"][data-measure-index-start="${measureIndexStart}"]`,
  )
}

Given('the cmd-click system fixture is loaded', async ({ page }) => {
  await loadFixture(page)
  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
  await page.waitForSelector(
    '[data-tag="part-label"][data-part-index="0"][data-measure-index-start="1"]',
    { timeout: 10_000 },
  )
  await primeMeasureSpans(page)
})

When("I Ctrl-click system 0's Melody part label", async ({ page }) => {
  const system0Melody = partLabel(page, 0, 0)
  await expect(system0Melody).toBeVisible({ timeout: 5_000 })

  const box = await stableBoundingBox(system0Melody)
  if (!box) throw new Error('Could not get bounding box for system 0 Melody.')

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.up()
  await page.keyboard.up('Control')
})

When(
  "I Ctrl-click system 0's Melody label then system 1's Melody label",
  async ({ page }) => {
    const system0Melody = partLabel(page, 0, 0)
    const system1Melody = partLabel(page, 0, 1)
    await expect(system0Melody).toBeVisible({ timeout: 5_000 })
    await expect(system1Melody).toBeVisible({ timeout: 5_000 })

    const startBox = await stableBoundingBox(system0Melody)
    const endBox = await stableBoundingBox(system1Melody)
    if (!startBox || !endBox) {
      throw new Error(
        'Could not get bounding boxes for the system 0/1 Melody labels.',
      )
    }

    await page.keyboard.down('Control')
    await clickAndClickSelect(
      page,
      startBox.x + startBox.width / 2,
      startBox.y + startBox.height / 2,
      endBox.x + endBox.width / 2,
      endBox.y + endBox.height / 2,
    )
    await page.keyboard.up('Control')
  },
)

Then(
  '{int} range-selected notes belong to part index {int}, as seen in part label cmd click selects whole system',
  async ({ page }, count: number, partIndex: number) => {
    await expect(
      page.locator(
        `[data-tag="note"][data-note-range-selected][data-part-index="${partIndex}"]`,
      ),
    ).toHaveCount(count)
  },
)

Then(
  '{int} notes are range-selected in total, as seen in part label cmd click selects whole system',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  "system 0's Melody label's click-target rect is marked range-active",
  async ({ page }) => {
    await expect(
      partLabel(page, 0, 0).locator(
        'rect[data-variant="part-label-click-target-rect"]',
      ),
    ).toHaveAttribute('data-part-label-range-active', '')
  },
)

Then(
  "system 0's Harmony label's click-target rect is marked range-active",
  async ({ page }) => {
    await expect(
      partLabel(page, 1, 0).locator(
        'rect[data-variant="part-label-click-target-rect"]',
      ),
    ).toHaveAttribute('data-part-label-range-active', '')
  },
)

Then(
  "system 1's Melody label's click-target rect is marked range-active",
  async ({ page }) => {
    await expect(
      partLabel(page, 0, 1).locator(
        'rect[data-variant="part-label-click-target-rect"]',
      ),
    ).toHaveAttribute('data-part-label-range-active', '')
  },
)

Then(
  "system 1's Harmony label's click-target rect is marked range-active",
  async ({ page }) => {
    await expect(
      partLabel(page, 1, 1).locator(
        'rect[data-variant="part-label-click-target-rect"]',
      ),
    ).toHaveAttribute('data-part-label-range-active', '')
  },
)
