import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click *part-label* range gesture not
 * spanning a system boundary — the part-label mirror of
 * `lyric-label-range-select-crosses-system.feature`. Range resolution
 * shouldn't know the concept of a "system" at all: a plain click-and-click
 * from a part label in one system to the same part's label in a later
 * system should select every note that part sounds in between, exactly like
 * `LyricLabel ↔ LyricLabel` already does across a system boundary (see
 * `resolve_selection_range_response` in `selection_range.rs`), with no
 * Cmd/Ctrl modifier required. Before this fix `PartLabel ↔ PartLabel` was
 * only ID-resolved for a same-system pair (guarded on matching
 * `measure_index_start`/`measure_index_end`); a cross-system pair fell
 * through to the pixel-marquee fallback (`partLabelsInMarquee`), which
 * restricts to the *anchor's own system* — so a plain click-and-click only
 * selected the first system's notes, dropping the second system's entirely.
 *
 * `break` forces measure 1 onto its own system regardless of
 * `max_measures_per_system`, so this fixture needs no filler content to
 * reproduce the boundary.
 */
const source = [
  '# metadata',
  'title = "cross system part label range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0 — system 0
  '',
  'break',
  '[M] 3 4', // measure 1 — system 1
].join('\n')

function partLabelInSystem(
  page: import('@playwright/test').Page,
  measureIndexStart: number,
) {
  return page.locator(
    `[data-tag="part-label"][data-part-index="0"][data-measure-index-start="${measureIndexStart}"]`,
  )
}

Given(
  'the cross-system part-label range-selection fixture is loaded and labels have rendered',
  async ({ page }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'cross-system-part-label-range-test.jianpu',
          userFiles: { 'cross-system-part-label-range-test.jianpu': src },
          bin: {},
          fileIds: {
            'cross-system-part-label-range-test.jianpu':
              'cross-system-part-label-range-test-id-001',
          },
        }),
      )
    }, source)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    // System 0's Melody label sits at measure-index-start 0, system 1's at
    // measure-index-start 1 — see `partLabelInSystem`.
    await page.waitForSelector(
      '[data-tag="part-label"][data-part-index="0"][data-measure-index-start="0"]',
      { timeout: 10_000 },
    )
    await page.waitForSelector(
      '[data-tag="part-label"][data-part-index="0"][data-measure-index-start="1"]',
      { timeout: 10_000 },
    )
  },
)

When(
  'I click-and-click select the Melody label in system 0 then the Melody label in system 1',
  async ({ page }) => {
    const fromLabel = partLabelInSystem(page, 0)
    const toLabel = partLabelInSystem(page, 1)
    await expect(fromLabel).toBeVisible({ timeout: 5_000 })
    await expect(toLabel).toBeVisible({ timeout: 5_000 })

    const fromBox = await stableBoundingBox(fromLabel)
    const toBox = await stableBoundingBox(toLabel)
    if (!fromBox || !toBox) {
      throw new Error(
        'Could not get bounding boxes for the cross-system Melody labels.',
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
  '{int} notes are range-selected in total, as seen in part label range select crosses system',
  async ({ page }, count: number) => {
    // Melody sounds 4 notes total across both systems ("1 2" + "3 4") — the
    // range should cover every one of them, not just the anchor's own
    // system.
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  '{int} range-selected notes belong to part index {int}, as seen in part label range select crosses system',
  async ({ page }, count: number, partIndex: number) => {
    await expect(
      page.locator(
        `[data-tag="note"][data-note-range-selected][data-part-index="${partIndex}"]`,
      ),
    ).toHaveCount(count)
  },
)
