import { expect } from '@playwright/test'
import { Given, Then } from './fixtures'

/**
 * Regression fixture for the click-and-click note-range gesture's new
 * cross-part row (see `resolve_selection_range_response`'s cross-part
 * `Note` arm in `crates/jianpu-wasm/src/selection_range.rs`, added per
 * `PLAN-clickable-element-id-selection.md`'s Phase 2): an anchor in one
 * part and a second click in another part now resolves via wasm — every
 * note whose part falls between the two `source_part_index`es AND whose
 * measure falls between the two endpoints' own `measure_index`es (looked
 * up from `note_spans`, not the click's pixel position). Two parts, two
 * measures each, so a sweep anchored in measure 0 of one part and
 * committed in measure 0 of the other part should pick up only that
 * measure's four notes (indices 0-3), not measure 1's (indices 4-7) —
 * confirming the measure range is genuinely bounded by the two clicks'
 * own measures, not "everything from here on."
 */
const crossPartSource = [
  '# metadata',
  'title = "cross part range test"',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0 — notes 0-1
  '[H] 5 6', // measure 0 — notes 2-3
  '',
  '[M] 3 4', // measure 1 — notes 4-5
  '[H] 7 1', // measure 1 — notes 6-7
].join('\n')

function noteRects(page: import('@playwright/test').Page) {
  return page.locator('rect[data-variant="note-click-target-rect"]')
}

// `data-note-id` restarts at 0 within each part (see `noteRects`' two note
// groups below), so it can't disambiguate across parts on its own — walk up
// from the nth click-target rect (in render order) to its own `[data-tag="note"]`
// group instead, the same "index" ordering `noteRects(page).nth(...)` already
// uses to pick the click point.
function noteGroup(page: import('@playwright/test').Page, index: number) {
  return noteRects(page)
    .nth(index)
    .locator('xpath=ancestor::*[@data-tag="note"][1]')
}

Given(
  'the cross-part range-selection fixture is loaded and note click targets have rendered',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'cross-part-range-test.jianpu',
          userFiles: { 'cross-part-range-test.jianpu': source },
          bin: {},
          fileIds: {
            'cross-part-range-test.jianpu': 'cross-part-range-test-id-001',
          },
        }),
      )
    }, crossPartSource)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="measure"][data-measure-index="1"]', {
      timeout: 10_000,
    })
    await expect(noteRects(page)).toHaveCount(8, { timeout: 10_000 })
  },
)

// Reuses the "I click-and-click select the note at index {int} then the
// note at index {int}" step already defined in
// `note-range-select-crosses-system.steps.ts` — playwright-bdd resolves
// step text against one project-wide registry, so redefining the identical
// pattern here would collide with it.

Then(
  'the notes at index {int}, {int}, {int} and {int} are all range-selected',
  async ({ page }, a: number, b: number, c: number, d: number) => {
    for (const noteId of [a, b, c, d]) {
      await expect(noteGroup(page, noteId)).toHaveAttribute(
        'data-note-range-selected',
        '',
      )
    }
  },
)

Then(
  'the notes at index {int}, {int}, {int} and {int} are not range-selected',
  async ({ page }, a: number, b: number, c: number, d: number) => {
    for (const noteId of [a, b, c, d]) {
      await expect(noteGroup(page, noteId)).not.toHaveAttribute(
        'data-note-range-selected',
        '',
      )
    }
  },
)
