import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Cmd/Ctrl-clicking (or click-and-clicking across) a measure in the SVG
 * preview selects every note/rest cell in that measure (see
 * `measure-click-selects-notes.spec.ts`) — and, alongside them, every lyric
 * syllable in that same measure, via `previewSelection.ts`'s
 * `lyricCellsInMeasureRange`. A plain click / click-and-click resolves to
 * note/chord/syllable granularity instead and never pulls in lyric cells
 * from a note hit (see `Preview.tsx`'s `onMouseDown`).
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" so all measures render in one row and stay within the viewport.
 *
 * Measure 0 : [M] 1 2 3 4 / do re mi fa   — 4 notes, 4 syllables
 * Measure 1 : [M] 5 6     / sol la        — 2 notes, 2 syllables
 */
const clickTestSource = [
  '# metadata',
  'title = "measure click lyric test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — line 9
  'do re mi fa', // line 10
  '',
  '[M] 5 6', // measure 1 — line 12
  'sol la', // line 13
].join('\n')

async function loadClickTestFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'measure-click-lyric-test.jianpu',
        userFiles: { 'measure-click-lyric-test.jianpu': source },
        bin: {},
        fileIds: {
          'measure-click-lyric-test.jianpu': 'measure-click-lyric-test-id-001',
        },
      }),
    )
  }, clickTestSource)
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

Given('the measure-click lyric test fixture is loaded', async ({ page }) => {
  await loadClickTestFixture(page)
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
  'I plain-click on the note row near the top of measure 1',
  async ({ page }) => {
    const measure1 = page
      .locator('[data-tag="measure"][data-measure-index="1"]')
      .first()
    await expect(measure1).toBeVisible({ timeout: 5_000 })
    const box = await stableBoundingBox(measure1)
    if (!box) throw new Error('Could not get bounding box for measure 1.')

    // A plain click (mousedown + mouseup at the same point, no drag). Clicks
    // near the top of the measure's click-target rect, on the note row rather
    // than the lyric row beneath it (a lyric syllable has its own, narrower
    // click target that sits on top of the note's — see
    // `note-click-target-excludes-lyric-row.spec.ts`).
    await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.25)
    await page.mouse.down()
    await page.mouse.up()
  },
)

When(
  'I Cmd\\/Ctrl-click on the note row near the top of measure 1',
  async ({ page }) => {
    const measure1 = page
      .locator('[data-tag="measure"][data-measure-index="1"]')
      .first()
    await expect(measure1).toBeVisible({ timeout: 5_000 })
    const box = await stableBoundingBox(measure1)
    if (!box) throw new Error('Could not get bounding box for measure 1.')

    // A Cmd/Ctrl-modified plain click (mousedown + mouseup at the same point,
    // no drag) — the only remaining way to reach whole-measure selection (see
    // `Preview.tsx`'s `onMouseDown`).
    await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.25)
    await page.keyboard.down('Control')
    await page.mouse.down()
    await page.mouse.up()
    await page.keyboard.up('Control')
  },
)

When(
  "I Cmd\\/Ctrl-click measure 0's note row then measure 1's note row",
  async ({ page }) => {
    const measure0 = page
      .locator('[data-tag="measure"][data-measure-index="0"]')
      .first()
    const measure1 = page
      .locator('[data-tag="measure"][data-measure-index="1"]')
      .first()
    await expect(measure0).toBeVisible({ timeout: 5_000 })
    await expect(measure1).toBeVisible({ timeout: 5_000 })

    const box0 = await stableBoundingBox(measure0)
    const box1 = await stableBoundingBox(measure1)
    if (!box0 || !box1) {
      throw new Error('Could not get bounding boxes for measures 0 and 1.')
    }

    // As above, click points on the note row rather than the lyric row —
    // this gesture's anchor point actually misses every note/lyric click
    // target (the lyric row's presence shifts the note row's own click-target
    // down slightly). It now falls back to whole-measure selection either
    // way (see `previewClickHandler.ts`'s `handleAnchorClick`), but Cmd/Ctrl
    // is kept here for parity with the general "select this measure"
    // gesture this test means to exercise.
    await page.keyboard.down('Control')
    await clickAndClickSelect(
      page,
      box0.x + 1,
      box0.y + 1,
      box1.x + box1.width - 1,
      box1.y + box1.height * 0.25,
    )
    await page.keyboard.up('Control')
  },
)

Then('{int} note is range-selected', async ({ page }, count: number) => {
  await expect(
    page.locator('[data-tag="note"][data-note-range-selected]'),
  ).toHaveCount(count)
})

Then(
  '{int} notes are range-selected, as seen in measure click selects lyrics',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then('{int} lyrics are range-selected', async ({ page }, count: number) => {
  await expect(
    page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
  ).toHaveCount(count)
})

Then(
  'lyrics with note ids {int} and {int} are range-selected',
  async ({ page }, a: number, b: number) => {
    for (const noteId of [a, b]) {
      await expect(
        page.locator(
          `[data-tag="lyric"][data-lyric-range-selected][data-note-id="${noteId}"]`,
        ),
      ).toHaveCount(1)
    }
  },
)
