import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Clicking (or click-and-click range-selecting vertically across) a lyric-verse label — the
 * abbreviation drawn at the label region's left edge on each verse's own
 * row, e.g. "M:v1" for Melody's first verse — is a shortcut for selecting
 * every syllable that verse sings across the whole system the label sits
 * in (see `Preview.tsx`'s `getLyricLabelAtPoint`/`lyricCellsForLyricLabels`).
 * The lyric-side mirror of `part-label-click-selects-notes.spec.ts`.
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" so both measures render in one system and every label stays
 * within the viewport.
 *
 * Measure 0: Melody "1 2", verse 1 "do re", verse 2 "fa sol"
 * Measure 1: Melody "3 4", verse 1 "la ti", verse 2 "da di"
 */
const source = [
  '# metadata',
  'title = "lyric label click test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0
  'do re', // verse 0
  'fa sol', // verse 1
  '',
  '[M] 3 4', // measure 1
  'la ti', // verse 0
  'da di', // verse 1
].join('\n')

function verseLabel(page: import('@playwright/test').Page, verse: number) {
  return page
    .locator(
      `[data-tag="lyric-label"][data-part-index="0"][data-verse="${verse}"]`,
    )
    .first()
}

function verseLabelRect(page: import('@playwright/test').Page, verse: number) {
  return verseLabel(page, verse).locator(
    'rect[data-variant="lyric-label-click-target-rect"]',
  )
}

Given(
  'the lyric label click test fixture is loaded and measure spans are primed',
  async ({ page, focusEditor }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'lyric-label-click-test.jianpu',
          userFiles: { 'lyric-label-click-test.jianpu': src },
          bin: {},
          fileIds: {
            'lyric-label-click-test.jianpu': 'lyric-label-click-test-id-001',
          },
        }),
      )
    }, source)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector(
      '[data-tag="lyric-label"][data-part-index="0"][data-verse="0"]',
      { timeout: 10_000 },
    )
    await page.waitForSelector(
      '[data-tag="lyric-label"][data-part-index="0"][data-verse="1"]',
      { timeout: 10_000 },
    )

    await focusEditor()
    await page.keyboard.press('Control+g')
    await page.keyboard.type('10')
    await page.keyboard.press('Enter')
    await expect(page.locator('button.play-measure-btn')).toHaveText(
      /Measure/,
      { timeout: 5_000 },
    )
    await expect(
      page.locator('.preview-page [data-testid="measure-highlight"]').first(),
    ).toBeVisible({ timeout: 5_000 })
  },
)

When(
  'I click the verse {int} lyric label without clicking a second time',
  async ({ page }, verse: number) => {
    const label = verseLabel(page, verse)
    await expect(label).toBeVisible({ timeout: 5_000 })
    const box = await stableBoundingBox(label)
    if (!box)
      throw new Error(
        `Could not get bounding box for the verse ${verse} label.`,
      )

    // A plain click (mousedown + mouseup at the same point, no drag).
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await page.mouse.down()
    await page.mouse.up()
  },
)

When(
  'I click the verse {int} lyric label then click the verse {int} lyric label',
  async ({ page }, fromVerse: number, toVerse: number) => {
    const fromLabel = verseLabel(page, fromVerse)
    const toLabel = verseLabel(page, toVerse)
    await expect(fromLabel).toBeVisible({ timeout: 5_000 })
    await expect(toLabel).toBeVisible({ timeout: 5_000 })

    const fromBox = await stableBoundingBox(fromLabel)
    const toBox = await stableBoundingBox(toLabel)
    if (!fromBox || !toBox) {
      throw new Error('Could not get bounding boxes for the verse labels.')
    }

    await clickAndClickSelect(
      page,
      fromBox.x + fromBox.width / 2,
      fromBox.y + fromBox.height / 2,
      toBox.x + toBox.width / 2,
      toBox.y + toBox.height / 2,
    )
  },
)

Then(
  "verse 0's 4 syllables are range-selected and verse 1's are not",
  async ({ page }) => {
    // Verse 1 sounds 4 syllables total across both measures ("do re" + "la
    // ti"); none of verse 2's syllables should be selected.
    const highlightedLyrics = page.locator(
      '[data-tag="lyric"][data-lyric-range-selected]',
    )
    await expect(highlightedLyrics).toHaveCount(4)
    await expect(
      page.locator(
        '[data-tag="lyric"][data-lyric-range-selected][data-verse="0"]',
      ),
    ).toHaveCount(4)
    await expect(
      page.locator(
        '[data-tag="lyric"][data-lyric-range-selected][data-verse="1"]',
      ),
    ).toHaveCount(0)
  },
)

Then(
  'the verse 0 label stays visually active but the verse 1 label does not',
  async ({ page }) => {
    // The clicked label stays visually selected after mouseup; the untouched
    // one never was.
    await expect(verseLabelRect(page, 0)).toHaveAttribute(
      'data-lyric-label-range-active',
      '',
    )
    await expect(verseLabelRect(page, 1)).not.toHaveAttribute(
      'data-lyric-label-range-active',
      '',
    )
  },
)

Then(
  "verse 0's and verse 1's syllables are all range-selected",
  async ({ page }) => {
    // Verse 1's 4 syllables + verse 2's 4 syllables = 8.
    const highlightedLyrics = page.locator(
      '[data-tag="lyric"][data-lyric-range-selected]',
    )
    await expect(highlightedLyrics).toHaveCount(8)
  },
)

Then(
  'both the verse 0 and verse 1 labels stay visually active',
  async ({ page }) => {
    await expect(verseLabelRect(page, 0)).toHaveAttribute(
      'data-lyric-label-range-active',
      '',
    )
    await expect(verseLabelRect(page, 1)).toHaveAttribute(
      'data-lyric-label-range-active',
      '',
    )
  },
)
