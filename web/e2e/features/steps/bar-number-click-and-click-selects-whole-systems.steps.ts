import { expect } from '@playwright/test'
import { clickThenStableClick } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * `max_measures_per_system = 2` packs two measures per system, so the
 * "escalate to the whole system" behavior (see 'bar-number-system' mode in
 * `previewAnchorState.ts`) is distinguishable from a plain measure-index
 * range — a second click landing on measure 2 (system 1's *first* measure)
 * must still pull in measure 3 (the rest of system 1), which a bare
 * measure-range union of [0, 2] would not:
 *
 *   System 0 (measures 0-1): Melody "1 2 3 4" / "la la la la", Harmony "5 6 7 1'"
 *   System 1 (measures 2-3): Melody "5 6 7 1'" / "la la la la", Harmony "1' 7 6 5"
 *
 * Melody note ids run 0-7 in written order (0,1 in measure 0; 2,3 in
 * measure 1; 4,5 in measure 2; 6,7 in measure 3) — Harmony's ids mirror the
 * same per-measure numbering independently.
 */
const source = [
  '# metadata',
  'title = "bar number click-and-click whole systems test"',
  'max_measures_per_system = 2',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0 — system 0
  'la la',
  '[H] 5 6',
  '',
  '[M] 3 4', // measure 1 — system 0
  'la la',
  "[H] 7 1'",
  '',
  '[M] 5 6', // measure 2 — system 1
  'la la',
  "[H] 1' 7",
  '',
  "[M] 7 1'", // measure 3 — system 1
  'la la',
  '[H] 6 5',
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'bar-number-click-and-click-whole-systems-test.jianpu',
        userFiles: {
          'bar-number-click-and-click-whole-systems-test.jianpu': source,
        },
        bin: {},
        fileIds: {
          'bar-number-click-and-click-whole-systems-test.jianpu':
            'bar-number-click-and-click-whole-systems-test-id-001',
        },
      }),
    )
  }, source)
}

/** Waits for measureSpans to be primed (same priming dance other
 * measure-select specs use) so the SVG has settled before hit-testing. */
async function primeMeasureSpans(page: import('@playwright/test').Page) {
  await focusEditor(page)
  await page.keyboard.press('Control+g')
  await page.keyboard.type('20')
  await page.keyboard.press('Enter')
  await expect(page.locator('button.play-measure-btn')).toHaveText(/Measure/, {
    timeout: 5_000,
  })
  await expect(
    page.locator('.preview-page [data-testid="measure-highlight"]').first(),
  ).toBeVisible({ timeout: 5_000 })
}

function barNumberRect(
  page: import('@playwright/test').Page,
  measureIndex: number,
) {
  return page
    .locator(`[data-tag="bar-number"][data-measure-index="${measureIndex}"]`)
    .locator('rect[data-variant="bar-number-click-target-rect"]')
    .first()
}

// `.first()` on both: the preview renders a plain and a highlighted SVG
// document during the priming dance's async swap (see
// `primeMeasureSpans`'s doc comment on other measure-select specs), so a
// note/lyric id can transiently resolve to two elements — same rationale as
// `barNumberRect`'s own `.first()` above.
function note(
  page: import('@playwright/test').Page,
  partIndex: number,
  noteId: number,
) {
  return page
    .locator(
      `[data-tag="note"][data-part-index="${partIndex}"][data-note-id="${noteId}"]`,
    )
    .first()
}

function lyric(
  page: import('@playwright/test').Page,
  partIndex: number,
  noteId: number,
  verse: number,
) {
  return page
    .locator(
      `[data-tag="lyric"][data-part-index="${partIndex}"][data-note-id="${noteId}"][data-verse="${verse}"]`,
    )
    .first()
}

Given(
  'the bar-number click-and-click whole-systems fixture is loaded',
  async ({ page }) => {
    await loadFixture(page)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector(
      '[data-tag="bar-number"][data-measure-index="2"]',
      { timeout: 10_000 },
    )
    await primeMeasureSpans(page)
  },
)

When(
  "I click-and-click select measure 0's bar number then measure 0's bar number",
  async ({ page }) => {
    await clickThenStableClick(
      page,
      barNumberRect(page, 0),
      barNumberRect(page, 0),
    )
  },
)

When(
  "I click-and-click select measure 0's bar number then the first note in measure 2's Melody notes",
  async ({ page }) => {
    // Measure 2 is Melody's 3rd measure — its first note is note id 4 (see
    // this file's own doc comment on note-id numbering).
    await clickThenStableClick(page, barNumberRect(page, 0), note(page, 0, 4))
  },
)

When(
  "I click-and-click select measure 0's bar number then a lyric syllable in measure 2",
  async ({ page }) => {
    // Same note id 4 as the note-anchored scenario above, but landing on its
    // lyric syllable's own click target instead of the note's.
    await clickThenStableClick(
      page,
      barNumberRect(page, 0),
      lyric(page, 0, 4, 0),
    )
  },
)

Then(
  '{int} range-selected notes belong to part index {int}, as seen in bar number click-and-click selects whole systems',
  async ({ page }, count: number, partIndex: number) => {
    await expect(
      page.locator(
        `[data-tag="note"][data-note-range-selected][data-part-index="${partIndex}"]`,
      ),
    ).toHaveCount(count)
  },
)

Then(
  '{int} notes are range-selected in total, as seen in bar number click-and-click selects whole systems',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  'the play-measure button reads Selection, as seen in bar number click-and-click selects whole systems',
  async ({ page }) => {
    await expect(page.locator('button.play-measure-btn')).toHaveText(
      /Selection/,
      { timeout: 3_000 },
    )
  },
)
