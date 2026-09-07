import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click *lyric* (syllable row) range
 * gesture crossing a part boundary — the cross-part counterpart to
 * `lyric-range-select-crosses-verse.feature`'s same-part case. Today
 * `Lyric ↔ Lyric` is only ID-resolved for a same-part pair; a different-part
 * pair falls through to `Err` and the caller falls back to 'lyric' mode's
 * pixel-marquee path — see `resolve_selection_range_response` in
 * `selection_range.rs` and `PLAN-clickable-element-id-selection.md`'s
 * "cross-part (any verse pairing)" writeup.
 *
 * No shared `note_id` axis across parts, so this reuses the cross-part
 * `Note ↔ Note`/`Note ↔ Lyric` arms' `measure_index`-range pattern instead,
 * plus its own `verse` range (both endpoints are `Lyric`, unlike
 * `Note ↔ Lyric`'s single lyric endpoint). Three measures, each part
 * carrying one verse: measure 2 is a distractor, outside the swept measure
 * range, proving the rule actually restricts by measure rather than
 * unioning every measure both parts appear in.
 */
const source = [
  '# metadata',
  'title = "cross part lyric range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  '[M] 1', // measure 0
  'do', // Melody verse 0
  '[H] 5',
  'la', // Harmony verse 0
  '',
  '[M] 2', // measure 1
  're',
  '[H] 6',
  'ti',
  '',
  '[M] 3', // measure 2 — distractor, outside the swept measure range
  'mi',
  '[H] 7',
  'sol',
].join('\n')

function lyricInPart(
  page: import('@playwright/test').Page,
  partIndex: number,
  verse: number,
) {
  return page.locator(
    `[data-tag="lyric"][data-part-index="${partIndex}"][data-verse="${verse}"]`,
  )
}

Given(
  'the cross-part lyric range-selection fixture is loaded and both parts have rendered',
  async ({ page }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'cross-part-lyric-range-test.jianpu',
          userFiles: { 'cross-part-lyric-range-test.jianpu': src },
          bin: {},
          fileIds: {
            'cross-part-lyric-range-test.jianpu':
              'cross-part-lyric-range-test-id-001',
          },
        }),
      )
    }, source)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await expect(lyricInPart(page, 0, 0)).toHaveCount(3, { timeout: 10_000 })
    await expect(lyricInPart(page, 1, 0)).toHaveCount(3, { timeout: 10_000 })
    await page.evaluate(() => document.fonts.ready)
    await page.waitForTimeout(200)
  },
)

When(
  "I click-and-click select Melody's verse 0 syllable {int} then Harmony's verse 0 syllable {int}",
  async ({ page }, fromIndex: number, toIndex: number) => {
    const fromLyric = lyricInPart(page, 0, 0).nth(fromIndex)
    const toLyric = lyricInPart(page, 1, 0).nth(toIndex)
    await expect(fromLyric).toBeVisible({ timeout: 5_000 })
    await expect(toLyric).toBeVisible({ timeout: 5_000 })

    const fromBox = await stableBoundingBox(fromLyric)
    const toBox = await stableBoundingBox(toLyric)
    if (!fromBox || !toBox) {
      throw new Error(
        'Could not get bounding boxes for the cross-part lyric syllables.',
      )
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
  "Melody's and Harmony's verse 0 syllables in measures {int} through {int} are range-selected",
  async ({ page }, measureStart: number, measureEnd: number) => {
    const selectedLyrics = page.locator(
      '[data-tag="lyric"][data-lyric-range-selected]',
    )
    const expectedCount = (measureEnd - measureStart + 1) * 2
    await expect(selectedLyrics).toHaveCount(expectedCount)
    for (const partIndex of [0, 1]) {
      await expect(
        page.locator(
          `[data-tag="lyric"][data-lyric-range-selected][data-part-index="${partIndex}"][data-verse="0"]`,
        ),
      ).toHaveCount(measureEnd - measureStart + 1)
    }
  },
)

Then(
  'no syllable in measure {int} is range-selected',
  async ({ page }, measureIndex: number) => {
    // Lyric groups carry no `data-measure-index` of their own (only
    // `data-part-index`/`data-note-id`/`data-verse` — see `Tag::Lyric` in
    // `src/serializer/mod.rs`), so identify measure 2's syllables by DOM
    // order instead: this fixture renders one syllable per measure per part,
    // in measure order, so index `measureIndex` picks out that measure's
    // syllable directly.
    for (const partIndex of [0, 1]) {
      await expect(
        lyricInPart(page, partIndex, 0).nth(measureIndex),
      ).not.toHaveAttribute('data-lyric-range-selected', '')
    }
  },
)
