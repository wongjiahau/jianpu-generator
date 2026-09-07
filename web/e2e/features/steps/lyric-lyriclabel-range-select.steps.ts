import { expect } from '@playwright/test'
import { clickThenStableClick } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click *lyric-anchored* gesture
 * resolving against a *lyric-label* `current` — one of the five label-mixed
 * combinations `PLAN-clickable-element-id-selection.md`'s Follow-up section
 * left open, now closed (see `resolve_selection_range_response`'s
 * `Lyric ↔ LyricLabel` arm in `selection_range.rs`). Before this, 'lyric'
 * mode's `resolveSelection` only tried `getLyricAtPoint`/`getNoteAtPoint`
 * for `current`, so a second click landing on a lyric label fell straight
 * to the pixel-marquee fallback.
 *
 * Both endpoints carry verse info here (unlike every other label-mixed
 * pair), so this ranges over `verse_range` too, alongside `part_range`/
 * `measure_range` — mirroring cross-part `Lyric ↔ Lyric`'s own
 * `verse_range` idea. `break` isolates each measure into its own system so
 * the label's span stays narrow — measure 2 is a distractor, outside the
 * swept range, on both verses.
 */
const source = [
  '# metadata',
  'title = "lyric lyriclabel range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1', // measure 0 — system 0
  'do', // verse 0
  'la', // verse 1
  '',
  'break',
  '[M] 2', // measure 1 — system 1
  're', // verse 0
  'ti', // verse 1
  '',
  'break',
  '[M] 3', // measure 2 — system 2, distractor
  'mi', // verse 0
  'sol', // verse 1
].join('\n')

function lyricInVerse(page: import('@playwright/test').Page, verse: number) {
  return page.locator(
    `[data-tag="lyric"][data-part-index="0"][data-verse="${verse}"]`,
  )
}

function verseLabelInSystem(
  page: import('@playwright/test').Page,
  verse: number,
  measureIndexStart: number,
) {
  return page.locator(
    `[data-tag="lyric-label"][data-part-index="0"][data-verse="${verse}"][data-measure-index-start="${measureIndexStart}"]`,
  )
}

Given(
  'the lyric-lyriclabel range-selection fixture is loaded and both verses have rendered',
  async ({ page }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'lyric-lyriclabel-range-test.jianpu',
          userFiles: { 'lyric-lyriclabel-range-test.jianpu': src },
          bin: {},
          fileIds: {
            'lyric-lyriclabel-range-test.jianpu':
              'lyric-lyriclabel-range-test-id-001',
          },
        }),
      )
    }, source)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await expect(lyricInVerse(page, 0)).toHaveCount(3, { timeout: 10_000 })
    await expect(lyricInVerse(page, 1)).toHaveCount(3, { timeout: 10_000 })
    await page.waitForSelector(
      '[data-tag="lyric-label"][data-part-index="0"][data-verse="1"][data-measure-index-start="1"]',
      { timeout: 10_000 },
    )
    await page.evaluate(() => document.fonts.ready)
    await page.waitForTimeout(200)
  },
)

When(
  "I click-and-click select verse 0's syllable in measure 0 then the verse 1 label in system 1",
  async ({ page }) => {
    const fromLyric = lyricInVerse(page, 0).nth(0)
    const toLabel = verseLabelInSystem(page, 1, 1)
    await expect(fromLyric).toBeVisible({ timeout: 5_000 })
    await expect(toLabel).toBeVisible({ timeout: 5_000 })

    await clickThenStableClick(page, fromLyric, toLabel)
  },
)

Then(
  '{int} syllables are range-selected in total, as seen in lyric lyriclabel range select',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  'no syllable in measure 2 is range-selected, as seen in lyric lyriclabel range select',
  async ({ page }) => {
    // Neither verse row carries a `data-measure-index` of its own — measure
    // 2's syllable is the third (index 2) in each verse's render order (one
    // syllable per measure per verse).
    for (const verse of [0, 1]) {
      await expect(lyricInVerse(page, verse).nth(2)).not.toHaveAttribute(
        'data-lyric-range-selected',
        '',
      )
    }
  },
)

Then(
  'no note is range-selected, as seen in lyric lyriclabel range select',
  async ({ page }) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(0)
  },
)
