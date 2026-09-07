import { expect } from '@playwright/test'
import { clickThenStableClick } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click *note-anchored* gesture
 * resolving against a *lyric-label* `current` — one of the five label-mixed
 * combinations `PLAN-clickable-element-id-selection.md`'s Follow-up section
 * left open, now closed (see `resolve_selection_range_response`'s
 * `Note ↔ LyricLabel` arm, backed by `resolve_note_like_lyric_like_range`,
 * in `selection_range.rs`).
 *
 * The `Note` side always contributes to `note_cells` (its own single
 * measure, widened against the label's `[start, end]` span); the
 * `LyricLabel` side always contributes to `lyric_cells`, restricted to its
 * own `verse` — proved here by Harmony carrying two verses, only one of
 * which (the label's own) ends up selected. `break` isolates each measure
 * into its own system; measure 2 is a distractor, outside the swept range.
 */
const source = [
  '# metadata',
  'title = "note lyriclabel range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  '[M] 1', // measure 0 — system 0
  '[H] 5',
  'do', // Harmony verse 0
  'fa', // Harmony verse 1 — proves the verse restriction, see below
  '',
  'break',
  '[M] 2', // measure 1 — system 1
  '[H] 6',
  're', // Harmony verse 0
  'sol', // Harmony verse 1
  '',
  'break',
  '[M] 3', // measure 2 — system 2, distractor
  '[H] 7',
  'mi', // Harmony verse 0
  'la', // Harmony verse 1
].join('\n')

// A note group carries a sibling `Tag::Note` group for its
// (pointer-events: none) playback-cursor rect alongside its own
// click-target rect, so a bare `[data-tag="note"]` query double-counts each
// note — filter to the click-target rect's own group, mirroring
// `note-range-select-crosses-part.steps.ts`'s `noteGroup` convention.
function noteInPart(page: import('@playwright/test').Page, partIndex: number) {
  return page
    .locator(`[data-tag="note"][data-part-index="${partIndex}"]`)
    .filter({
      has: page.locator('rect[data-variant="note-click-target-rect"]'),
    })
}

function lyricInVerse(
  page: import('@playwright/test').Page,
  partIndex: number,
  verse: number,
) {
  return page.locator(
    `[data-tag="lyric"][data-part-index="${partIndex}"][data-verse="${verse}"]`,
  )
}

function verseLabelInSystem(
  page: import('@playwright/test').Page,
  partIndex: number,
  verse: number,
  measureIndexStart: number,
) {
  return page.locator(
    `[data-tag="lyric-label"][data-part-index="${partIndex}"][data-verse="${verse}"][data-measure-index-start="${measureIndexStart}"]`,
  )
}

Given(
  'the note-lyriclabel range-selection fixture is loaded and both parts have rendered',
  async ({ page }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'note-lyriclabel-range-test.jianpu',
          userFiles: { 'note-lyriclabel-range-test.jianpu': src },
          bin: {},
          fileIds: {
            'note-lyriclabel-range-test.jianpu':
              'note-lyriclabel-range-test-id-001',
          },
        }),
      )
    }, source)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await expect(noteInPart(page, 0)).toHaveCount(3, { timeout: 10_000 })
    await expect(noteInPart(page, 1)).toHaveCount(3, { timeout: 10_000 })
    await expect(lyricInVerse(page, 1, 0)).toHaveCount(3, { timeout: 10_000 })
    await expect(lyricInVerse(page, 1, 1)).toHaveCount(3, { timeout: 10_000 })
    await page.waitForSelector(
      '[data-tag="lyric-label"][data-part-index="1"][data-verse="0"][data-measure-index-start="1"]',
      { timeout: 10_000 },
    )
    await page.evaluate(() => document.fonts.ready)
    await page.waitForTimeout(200)
  },
)

When(
  "I click-and-click select Melody's note in measure 0 then Harmony's verse label in system 1",
  async ({ page }) => {
    const fromNote = noteInPart(page, 0).nth(0)
    const toLabel = verseLabelInSystem(page, 1, 0, 1)
    await expect(fromNote).toBeVisible({ timeout: 5_000 })
    await expect(toLabel).toBeVisible({ timeout: 5_000 })

    await clickThenStableClick(page, fromNote, toLabel)
  },
)

Then(
  '{int} notes are range-selected in total, as seen in note lyriclabel range select',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  '{int} syllables are range-selected in total, as seen in note lyriclabel range select',
  async ({ page }, count: number) => {
    // Only Harmony's verse 0 (the label's own verse) should be selected —
    // verse 1 sits in range but on the wrong verse.
    const selected = page.locator(
      '[data-tag="lyric"][data-lyric-range-selected]',
    )
    await expect(selected).toHaveCount(count)
    await expect(
      page.locator(
        '[data-tag="lyric"][data-lyric-range-selected][data-part-index="1"][data-verse="1"]',
      ),
    ).toHaveCount(0)
  },
)

Then(
  'no note in measure 2 is range-selected, as seen in note lyriclabel range select',
  async ({ page }) => {
    for (const partIndex of [0, 1]) {
      await expect(noteInPart(page, partIndex).nth(2)).not.toHaveAttribute(
        'data-note-range-selected',
        '',
      )
    }
  },
)
