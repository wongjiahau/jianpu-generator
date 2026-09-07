import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click *lyric* (syllable row, not the
 * `LyricLabel` row) range gesture crossing a verse boundary — the same-part
 * counterpart to `lyric-range-select-crosses-part.feature`'s cross-part
 * case. Today `Lyric ↔ Lyric` is only ID-resolved for a same-part,
 * same-verse pair (guarded on matching `verse`); a different-verse pair
 * falls through to `Err` and the caller falls back to 'lyric' mode's
 * pixel-marquee path — see `resolve_selection_range_response` in
 * `selection_range.rs` and `PLAN-clickable-element-id-selection.md`'s
 * "cross-verse (same part)" writeup.
 *
 * `note_id` numbering is shared across a part's verses (the same fact the
 * same-part `Note ↔ Lyric` arm already leans on), so the rule ranges by
 * `note_id` alongside a `verse` range — a small two-axis grid, not the
 * fuller row/column model the plan's Open Questions still flag as
 * unbuilt. One measure of four notes, three verse lines: verse 2 and note
 * id 3 are both left outside the swept range as distractors, proving the
 * rule actually restricts on both axes rather than unioning every verse or
 * every note.
 */
const source = [
  '# metadata',
  'title = "cross verse lyric range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — note ids 0-3
  'do re mi fa', // verse 0
  'la ti fa sol', // verse 1
  'ho ho ho ho', // verse 2 — distractor, outside the swept verse range
].join('\n')

function lyricInVerse(
  page: import('@playwright/test').Page,
  verse: number,
  noteId: number,
) {
  return page.locator(
    `[data-tag="lyric"][data-part-index="0"][data-verse="${verse}"][data-note-id="${noteId}"]`,
  )
}

Given(
  'the cross-verse lyric range-selection fixture is loaded and both verses have rendered',
  async ({ page }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'cross-verse-lyric-range-test.jianpu',
          userFiles: { 'cross-verse-lyric-range-test.jianpu': src },
          bin: {},
          fileIds: {
            'cross-verse-lyric-range-test.jianpu':
              'cross-verse-lyric-range-test-id-001',
          },
        }),
      )
    }, source)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await expect(
      page.locator('[data-tag="lyric"][data-part-index="0"]'),
    ).toHaveCount(12, { timeout: 10_000 })
    await page.evaluate(() => document.fonts.ready)
    await page.waitForTimeout(200)
  },
)

When(
  "I click-and-click select verse {int}'s syllable {int} then verse {int}'s syllable {int}",
  async (
    { page },
    fromVerse: number,
    fromNoteId: number,
    toVerse: number,
    toNoteId: number,
  ) => {
    const fromLyric = lyricInVerse(page, fromVerse, fromNoteId)
    const toLyric = lyricInVerse(page, toVerse, toNoteId)
    await expect(fromLyric).toBeVisible({ timeout: 5_000 })
    await expect(toLyric).toBeVisible({ timeout: 5_000 })

    const fromBox = await stableBoundingBox(fromLyric)
    const toBox = await stableBoundingBox(toLyric)
    if (!fromBox || !toBox) {
      throw new Error(
        'Could not get bounding boxes for the cross-verse lyric syllables.',
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
  "verses {int} and {int}'s syllables with note id {int} through {int} are range-selected",
  async (
    { page },
    verseA: number,
    verseB: number,
    noteIdStart: number,
    noteIdEnd: number,
  ) => {
    const selectedLyrics = page.locator(
      '[data-tag="lyric"][data-lyric-range-selected]',
    )
    const expectedCount = (verseB - verseA + 1) * (noteIdEnd - noteIdStart + 1)
    await expect(selectedLyrics).toHaveCount(expectedCount)
    for (const verse of [verseA, verseB]) {
      for (let noteId = noteIdStart; noteId <= noteIdEnd; noteId++) {
        await expect(
          page.locator(
            `[data-tag="lyric"][data-lyric-range-selected][data-verse="${verse}"][data-note-id="${noteId}"]`,
          ),
        ).toHaveCount(1)
      }
    }
  },
)

Then(
  "verse {int}'s syllables are not range-selected",
  async ({ page }, verse: number) => {
    await expect(
      page.locator(
        `[data-tag="lyric"][data-lyric-range-selected][data-verse="${verse}"]`,
      ),
    ).toHaveCount(0)
  },
)

Then(
  'no syllable with note id {int} is range-selected',
  async ({ page }, noteId: number) => {
    await expect(
      page.locator(
        `[data-tag="lyric"][data-lyric-range-selected][data-note-id="${noteId}"]`,
      ),
    ).toHaveCount(0)
  },
)
