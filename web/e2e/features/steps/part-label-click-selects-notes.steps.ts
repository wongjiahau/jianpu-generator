import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" so both measures render in one system and both part labels stay
 * within the viewport.
 *
 * Measure 0: Melody "1 2" (2 notes), Harmony "5 6" (2 notes)
 * Measure 1: Melody "3 4" (2 notes), Harmony "7 1'" (2 notes)
 */
const source = [
  '# metadata',
  'title = "part label click test"',
  'max_measures_per_system = 48',
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
        active: 'part-label-click-test.jianpu',
        userFiles: { 'part-label-click-test.jianpu': source },
        bin: {},
        fileIds: {
          'part-label-click-test.jianpu': 'part-label-click-test-id-001',
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

function melodyLabel(page: import('@playwright/test').Page) {
  return page.locator('[data-tag="part-label"][data-part-index="0"]').first()
}

function harmonyLabel(page: import('@playwright/test').Page) {
  return page.locator('[data-tag="part-label"][data-part-index="1"]').first()
}

Given('the two-part click-test fixture is loaded', async ({ page }) => {
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

Given(
  'the notes-with-lyrics click-test fixture is loaded',
  async ({ page }) => {
    // Regression test: 'part-label' range-selection unions in the lyric row
    // underneath the swept part(s) — a real feature for click-and-click (see
    // `part-label-range-select-lyrics.spec.ts`) — but a plain click (zero
    // pointer movement) used to go through that exact same code path and
    // incorrectly pick up the lyric row too.
    const lyricSource = [
      '# metadata',
      'title = "part label click no-lyric test"',
      'max_measures_per_system = 48',
      '',
      '# parts',
      'Melody [M] = notes',
      '',
      '# score',
      '[M] 1 2', // measure 0
      'do re', // verse 0
      '',
      '[M] 3 4', // measure 1
      'mi fa', // verse 0
    ].join('\n')

    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'part-label-click-no-lyric-test.jianpu',
          userFiles: { 'part-label-click-no-lyric-test.jianpu': source },
          bin: {},
          fileIds: {
            'part-label-click-no-lyric-test.jianpu':
              'part-label-click-no-lyric-test-id-001',
          },
        }),
      )
    }, lyricSource)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="part-label"][data-part-index="0"]', {
      timeout: 10_000,
    })
    await primeMeasureSpans(page)
  },
)

Given(
  'the single-measure notes-with-lyrics fixture is loaded',
  async ({ page }) => {
    // Regression test: the part label's click-target rect used to absorb its
    // lyric verse row's height too, fully overlapping the `lyric-label` rect
    // one row down — since `:hover` paints the whole rect, hovering the note
    // label visually painted over the lyric label too.
    const lyricSource = [
      '# metadata',
      'title = "part label no lyric-label overlap test"',
      'max_measures_per_system = 48',
      '',
      '# parts',
      'Melody [M] = notes',
      '',
      '# score',
      '[M] 1 2', // measure 0
      'do re', // verse 0
    ].join('\n')

    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'part-label-no-overlap-test.jianpu',
          userFiles: { 'part-label-no-overlap-test.jianpu': source },
          bin: {},
          fileIds: {
            'part-label-no-overlap-test.jianpu':
              'part-label-no-overlap-test-id-001',
          },
        }),
      )
    }, lyricSource)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="part-label"][data-part-index="0"]')
    await primeMeasureSpans(page)
  },
)

When('I plain-click the Melody part label', async ({ page }) => {
  const label = melodyLabel(page)
  await expect(label).toBeVisible({ timeout: 5_000 })
  const box = await stableBoundingBox(label)
  if (!box) throw new Error('Could not get bounding box for the Melody label.')

  // A plain click (mousedown + mouseup at the same point, no drag).
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.mouse.down()
  await page.mouse.up()
})

When(
  'I click the Melody part label then click the Harmony part label',
  async ({ page }) => {
    const melody = melodyLabel(page)
    const harmony = harmonyLabel(page)
    await expect(melody).toBeVisible({ timeout: 5_000 })
    await expect(harmony).toBeVisible({ timeout: 5_000 })

    const melodyBox = await stableBoundingBox(melody)
    const harmonyBox = await stableBoundingBox(harmony)
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

When(
  'I wait 700ms for the multicursor debounce and worker round-trip',
  async ({ page }) => {
    // Click-and-click range-selecting notes pushes a Monaco multicursor
    // selection, whose cursor-change listener debounces (300 ms) into a worker
    // round-trip that swaps the plain SVG documents for highlighted ones —
    // the highlight (both the notes' and the part labels') must survive that
    // swap.
    await page.waitForTimeout(700)
  },
)

When(
  'I hover the Melody part label without clicking it',
  async ({ page }) => {
    const melody = melodyLabel(page)
    const melodyBox = await stableBoundingBox(melody)
    if (!melodyBox) {
      throw new Error('Could not get bounding box for the Melody label.')
    }
    // Hover the Melody label (no click yet) and record the fill its
    // `:hover` rule paints — this is the "hovered" look the anchor label
    // must keep showing for the rest of the gesture.
    await page.mouse.move(
      melodyBox.x + melodyBox.width / 2,
      melodyBox.y + melodyBox.height / 2,
    )
  },
)

When(
  'I click the Melody label and move the pointer onto the Harmony label without clicking it',
  async ({ page }) => {
    const melody = melodyLabel(page)
    const harmony = harmonyLabel(page)
    const melodyBox = await stableBoundingBox(melody)
    const harmonyBox = await stableBoundingBox(harmony)
    if (!melodyBox || !harmonyBox) {
      throw new Error('Could not get bounding boxes for the part labels.')
    }

    // Click Melody to anchor the gesture, then hover the pointer onto Harmony
    // without a second click yet — Melody is the label the gesture anchored
    // on, and per the vertical-sweep part-label shortcut it stays part of
    // the selection while the pointer is anywhere in the column, not just
    // while literally over its own rect.
    await page.mouse.down()
    await page.mouse.up()
    await page.mouse.move(
      harmonyBox.x + harmonyBox.width / 2,
      harmonyBox.y + harmonyBox.height / 2,
      { steps: 10 },
    )
  },
)

Then(
  '{int} notes are range-selected in total',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  '{int} range-selected notes belong to part index {int}',
  async ({ page }, count: number, partIndex: number) => {
    await expect(
      page.locator(
        `[data-tag="note"][data-note-range-selected][data-part-index="${partIndex}"]`,
      ),
    ).toHaveCount(count)
  },
)

Then(
  '{int} lyrics are range-selected in total',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
    ).toHaveCount(count)
  },
)

Then('the play button shows {string}', async ({ page }, text: string) => {
  await expect(page.locator('button.play-measure-btn')).toHaveText(
    new RegExp(text),
    { timeout: 3_000 },
  )
})

Then(
  "the Melody label's click-target rect is marked range-active",
  async ({ page }) => {
    await expect(
      melodyLabel(page).locator(
        'rect[data-variant="part-label-click-target-rect"]',
      ),
    ).toHaveAttribute('data-part-label-range-active', '')
  },
)

Then(
  "the Melody label's click-target rect is not marked range-active",
  async ({ page }) => {
    await expect(
      melodyLabel(page).locator(
        'rect[data-variant="part-label-click-target-rect"]',
      ),
    ).not.toHaveAttribute('data-part-label-range-active', '')
  },
)

Then(
  "the Harmony label's click-target rect is marked range-active",
  async ({ page }) => {
    await expect(
      harmonyLabel(page).locator(
        'rect[data-variant="part-label-click-target-rect"]',
      ),
    ).toHaveAttribute('data-part-label-range-active', '')
  },
)

Then(
  "the Harmony label's click-target rect is not marked range-active",
  async ({ page }) => {
    await expect(
      harmonyLabel(page).locator(
        'rect[data-variant="part-label-click-target-rect"]',
      ),
    ).not.toHaveAttribute('data-part-label-range-active', '')
  },
)

Then(
  "the part label's click-target rect does not vertically overlap the lyric label's click-target rect",
  async ({ page }) => {
    const partLabelRect = page
      .locator(
        '[data-tag="part-label"][data-part-index="0"] rect[data-variant="part-label-click-target-rect"]',
      )
      .first()
    const lyricLabelRect = page
      .locator(
        '[data-tag="lyric-label"][data-part-index="0"][data-verse="0"] rect[data-variant="lyric-label-click-target-rect"]',
      )
      .first()
    const partBox = await stableBoundingBox(partLabelRect)
    const lyricBox = await stableBoundingBox(lyricLabelRect)
    if (!partBox || !lyricBox) {
      throw new Error('Could not get bounding boxes for the label rects.')
    }

    // No vertical overlap between the two rects.
    expect(partBox.y + partBox.height).toBeLessThanOrEqual(lyricBox.y + 0.5)
  },
)

let hoveredFill = ''

Then(
  "the Melody label's click-target rect has a visible hover fill",
  async ({ page }) => {
    const melodyRect = melodyLabel(page).locator(
      'rect[data-variant="part-label-click-target-rect"]',
    )
    hoveredFill = await melodyRect.evaluate((el) => getComputedStyle(el).fill)
    expect(hoveredFill).not.toBe('none')
    expect(hoveredFill).not.toMatch(/^rgba\(0, ?0, ?0, ?0\)$/)
  },
)

Then(
  "the Melody label's click-target rect keeps the same hover fill while the second click is pending",
  async ({ page }) => {
    const melodyRect = melodyLabel(page).locator(
      'rect[data-variant="part-label-click-target-rect"]',
    )
    const fillWhilePending = await melodyRect.evaluate(
      (el) => getComputedStyle(el).fill,
    )
    expect(fillWhilePending).toBe(hoveredFill)
  },
)
