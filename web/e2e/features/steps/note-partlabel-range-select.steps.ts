import { expect } from '@playwright/test'
import { clickThenStableClick } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click *note-anchored* gesture
 * resolving against a *part-label* `current` — one of the five label-mixed
 * combinations `PLAN-clickable-element-id-selection.md`'s Follow-up section
 * left open, now closed (see `resolve_selection_range_response`'s
 * `Note ↔ PartLabel` arm in `selection_range.rs`). Before this, 'note'
 * mode's `resolveSelection` only tried `getNoteAtPoint`/`getLyricAtPoint`
 * for `current`, so a second click landing on a part label fell straight to
 * the "current missed every click target" pixel-marquee fallback, which
 * (via `applyNoteRangeSelection`'s own `getNoteAtPoint` re-check) resolves
 * to nothing.
 *
 * Neither endpoint carries verse info, so this reuses `PartLabel ↔
 * PartLabel`'s own rule: `part_range` from both endpoints' parts,
 * `measure_range` from the note's own (single) measure widened against the
 * label's `[start, end]` span. `break` isolates each measure into its own
 * system so the label's span stays narrow (just its own system, not the
 * whole score) — measure 2 is a distractor, outside the swept range,
 * proving the rule actually restricts by measure rather than unioning every
 * measure either part appears in.
 */
const source = [
  '# metadata',
  'title = "note partlabel range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  '[M] 1', // measure 0 — system 0
  '[H] 5',
  '',
  'break',
  '[M] 2', // measure 1 — system 1
  '[H] 6',
  '',
  'break',
  '[M] 3', // measure 2 — system 2, distractor
  '[H] 7',
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

function partLabelInSystem(
  page: import('@playwright/test').Page,
  partIndex: number,
  measureIndexStart: number,
) {
  return page.locator(
    `[data-tag="part-label"][data-part-index="${partIndex}"][data-measure-index-start="${measureIndexStart}"]`,
  )
}

Given(
  'the note-partlabel range-selection fixture is loaded and both parts have rendered',
  async ({ page }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'note-partlabel-range-test.jianpu',
          userFiles: { 'note-partlabel-range-test.jianpu': src },
          bin: {},
          fileIds: {
            'note-partlabel-range-test.jianpu':
              'note-partlabel-range-test-id-001',
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
    await page.waitForSelector(
      '[data-tag="part-label"][data-part-index="1"][data-measure-index-start="1"]',
      { timeout: 10_000 },
    )
  },
)

When(
  "I click-and-click select Melody's note in measure 0 then Harmony's label in system 1",
  async ({ page }) => {
    const fromNote = noteInPart(page, 0).nth(0)
    const toLabel = partLabelInSystem(page, 1, 1)
    await expect(fromNote).toBeVisible({ timeout: 5_000 })
    await expect(toLabel).toBeVisible({ timeout: 5_000 })

    await clickThenStableClick(page, fromNote, toLabel)
  },
)

Then(
  '{int} notes are range-selected in total, as seen in note partlabel range select',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  'no note in measure 2 is range-selected, as seen in note partlabel range select',
  async ({ page }) => {
    // Neither part carries a `data-measure-index` on its note group — measure
    // 2's note is the third (index 2) in each part's render order (one note
    // per measure per part).
    for (const partIndex of [0, 1]) {
      await expect(noteInPart(page, partIndex).nth(2)).not.toHaveAttribute(
        'data-note-range-selected',
        '',
      )
    }
  },
)
