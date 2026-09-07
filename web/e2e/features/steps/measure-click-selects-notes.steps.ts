import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * A plain click (or click-and-click) in the SVG preview resolves to
 * note/chord/syllable granularity — clicking a note selects just that note.
 * Selecting every note/rest cell in a whole measure at once now requires
 * holding Cmd/Ctrl (see `Preview.tsx`'s `onMouseDown`, which gates
 * `'measure'` mode behind `e.metaKey || e.ctrlKey`). Both paths reuse the
 * same note range-select highlight (`[data-note-range-selected]`) and Monaco
 * multicursor pathway (`onNoteRangeSelect`) that click-and-click sweeping a
 * marquee over individual notes uses.
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" so all measures render in one row and stay within the viewport.
 *
 * Measure 0 : [M] 1 2 3 4   — 4 notes
 * Measure 1 : [M] 5 6       — 2 notes
 * Measure 2 : [M] 7 1'      — 2 notes
 */
const clickTestSource = [
  '# metadata',
  'title = "measure click test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — line 9
  '',
  '[M] 5 6', // measure 1 — line 11
  '',
  "[M] 7 1'", // measure 2 — line 13
].join('\n')

async function loadClickTestFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'measure-click-test.jianpu',
        userFiles: { 'measure-click-test.jianpu': source },
        bin: {},
        fileIds: { 'measure-click-test.jianpu': 'measure-click-test-id-001' },
      }),
    )
  }, clickTestSource)
}

/** Waits for measureSpans to be primed (same priming dance the old
 * measure-select specs used) so the SVG has settled before hit-testing. */
async function primeMeasureSpans(page: import('@playwright/test').Page) {
  await focusEditor(page)
  await page.keyboard.press('Control+g')
  await page.keyboard.type('9')
  await page.keyboard.press('Enter')
  await expect(page.locator('button.play-measure-btn')).toHaveText(/Measure/, {
    timeout: 5_000,
  })
  // Priming the cursor also triggers an async highlight re-render that swaps
  // the SVG DOM and scrolls it into view — wait for that to settle before
  // measuring positions, otherwise bounding boxes captured mid-scroll are
  // inconsistent with each other.
  await expect(
    page.locator('.preview-page [data-testid="measure-highlight"]').first(),
  ).toBeVisible({ timeout: 5_000 })
}

Given('the measure-click test fixture is loaded', async ({ page }) => {
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

When('I plain-click the center of measure 1', async ({ page }) => {
  const measure1 = page
    .locator('[data-tag="measure"][data-measure-index="1"]')
    .first()
  await expect(measure1).toBeVisible({ timeout: 5_000 })
  const box = await stableBoundingBox(measure1)
  if (!box) throw new Error('Could not get bounding box for measure 1.')

  // A plain click (mousedown + mouseup at the same point, no second click).
  // Notes fully tile their measure's width (no gaps between click-target
  // rects — see the click-and-click test below), so this always lands on
  // exactly one of measure 1's two notes.
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.mouse.down()
  await page.mouse.up()
})

When('I Cmd\\/Ctrl-click the center of measure 1', async ({ page }) => {
  const measure1 = page
    .locator('[data-tag="measure"][data-measure-index="1"]')
    .first()
  await expect(measure1).toBeVisible({ timeout: 5_000 })
  const box = await stableBoundingBox(measure1)
  if (!box) throw new Error('Could not get bounding box for measure 1.')

  // A Cmd/Ctrl-modified plain click (mousedown + mouseup at the same point,
  // no drag) — the only remaining way to reach whole-measure selection (see
  // `Preview.tsx`'s `onMouseDown`).
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.up()
  await page.keyboard.up('Control')
})

When("I Cmd\\/Ctrl-click measure 1's own left edge pixel", async ({ page }) => {
  const measure1 = page
    .locator('[data-tag="measure"][data-measure-index="1"]')
    .first()
  await expect(measure1).toBeVisible({ timeout: 5_000 })
  const box = await stableBoundingBox(measure1)
  if (!box) throw new Error('Could not get bounding box for measure 1.')

  // Click exactly at measure 1's own reported left edge — the pixel where
  // measure 0's and measure 1's click-target rects meet. `getMeasureAtPoint`
  // must resolve this to measure 1 (the measure that pixel is reported as
  // belonging to), not measure 0: at a coincident rect edge,
  // `elementsFromPoint`'s z-order is not a reliable tie-break (see
  // `Preview.tsx`'s `getMeasureAtPoint`), which previously made this click
  // resolve to the wrong (previous) measure. Held under Cmd/Ctrl since a
  // plain click no longer goes through `getMeasureAtPoint` at all when it
  // lands on a note's own click target.
  await page.mouse.move(box.x, box.y + box.height / 2)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.up()
  await page.keyboard.up('Control')
})

When(
  'I click corner-to-corner from measure 0 to measure 2',
  async ({ page }) => {
    await page.waitForSelector('[data-tag="measure"][data-measure-index="2"]', {
      timeout: 10_000,
    })

    const measure0 = page
      .locator('[data-tag="measure"][data-measure-index="0"]')
      .first()
    const measure2 = page
      .locator('[data-tag="measure"][data-measure-index="2"]')
      .first()
    await expect(measure0).toBeVisible({ timeout: 5_000 })
    await expect(measure2).toBeVisible({ timeout: 5_000 })

    const box0 = await stableBoundingBox(measure0)
    const box2 = await stableBoundingBox(measure2)
    if (!box0 || !box2) {
      throw new Error('Could not get bounding boxes for measures 0 and 2.')
    }

    // Notes fully tile their measure's width (no gaps between click-target
    // rects), so a click anywhere inside a measure always lands on some note
    // and this gesture follows the raw note-marquee path (see
    // `previewClickHandler.ts`'s 'note' mode) — click corner-to-corner rather
    // than center-to-center so the marquee's bounding box fully covers every
    // note between measure 0 and measure 2, not just the ones between their
    // two center points.
    await clickAndClickSelect(
      page,
      box0.x + 1,
      box0.y + 1,
      box2.x + box2.width - 1,
      box2.y + box2.height - 1,
    )
  },
)

Then(
  '{int} note is range-selected, as seen in measure click selects notes',
  async ({ page }, count: number) => {
    const highlightedNotes = page.locator(
      '[data-tag="note"][data-note-range-selected]',
    )
    await expect(highlightedNotes).toHaveCount(count)
  },
)

Then(
  '{int} notes are range-selected, as seen in measure click selects notes',
  async ({ page }, count: number) => {
    const highlightedNotes = page.locator(
      '[data-tag="note"][data-note-range-selected]',
    )
    await expect(highlightedNotes).toHaveCount(count)
  },
)

Then(
  'note ids {int} and {int} are range-selected, as seen in measure click selects notes',
  async ({ page }, a, b) => {
    for (const noteId of [a, b]) {
      await expect(
        page.locator(
          `[data-tag="note"][data-note-range-selected][data-note-id="${noteId}"]`,
        ),
      ).toHaveCount(1)
    }
  },
)

Then(
  'the play-measure button reads Selection, as seen in measure click selects notes',
  async ({ page }) => {
    await expect(page.locator('button.play-measure-btn')).toHaveText(
      /Selection/,
      { timeout: 3_000 },
    )
  },
)

Then(
  'the selection survives the debounced highlight swap with {int} notes still selected',
  async ({ page }, count: number) => {
    // Click-and-click range-selecting notes pushes a Monaco multicursor selection,
    // whose cursor-change listener debounces (300 ms) into a worker
    // round-trip that swaps the plain SVG documents for highlighted ones —
    // the highlight must survive that swap.
    await page.waitForTimeout(700)
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

/**
 * Self-contained source with a run of 3 consecutive all-rest measures
 * (measures 1-3), which the renderer collapses into a single wide
 * multi-measure-rest bar (see "Multi-measure rests" in syntax.md).
 *
 * Measure 0 : [M] 1 1 1 1        — normal, not part of the rest run
 * Measure 1 : [M] 0 0 0 0        — rest, start of the merged run
 * Measure 2 : [M] 0 0 0 0        — rest, middle of the merged run
 * Measure 3 : [M] 0 0 0 0        — rest, end of the merged run
 * Measure 4 : [M] 2 2 2 2        — normal, not part of the rest run
 *
 * The merged run compiles down to a single rest note/rest cell (one
 * `note_id` spanning the whole run — see `group_elements_by_note_id`), not
 * one cell per source measure, so clicking anywhere on the merged bar must
 * select that one cell, whose source span covers all three measures.
 */
const mergedRestSource = [
  '# metadata',
  'title = "merged rest click test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 1 1 1', // measure 0 — line 9
  '',
  '[M] 0 0 0 0', // measure 1 — line 11
  '',
  '[M] 0 0 0 0', // measure 2 — line 13
  '',
  '[M] 0 0 0 0', // measure 3 — line 15
  '',
  '[M] 2 2 2 2', // measure 4 — line 17
].join('\n')

Given('the merged-rest test fixture is loaded', async ({ page }) => {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'merged-rest-test.jianpu',
        userFiles: { 'merged-rest-test.jianpu': source },
        bin: {},
        fileIds: { 'merged-rest-test.jianpu': 'merged-rest-test-id-001' },
      }),
    )
  }, mergedRestSource)

  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })

  // The merged run (measures 1-3) renders as a single bar whose click target
  // carries measure_index=1 (the run's first source measure) and
  // measure_index_end=3 (the run's last source measure).
  const mergedBar = page.locator(
    '[data-tag="measure"][data-measure-index="1"][data-measure-index-end="3"]',
  )
  await expect(mergedBar.first()).toBeVisible({ timeout: 10_000 })
  await primeMeasureSpans(page)
})

When(
  'I plain-click the center of the merged rest bar spanning measures 1 to 3',
  async ({ page }) => {
    const mergedBar = page.locator(
      '[data-tag="measure"][data-measure-index="1"][data-measure-index-end="3"]',
    )
    const box = await stableBoundingBox(mergedBar.first())
    if (!box) {
      throw new Error('Could not get bounding box for the merged rest bar.')
    }

    // A plain click (mousedown + mouseup at the same point, no drag).
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await page.mouse.down()
    await page.mouse.up()
  },
)
