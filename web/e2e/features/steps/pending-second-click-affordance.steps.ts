import { expect } from '@playwright/test'
import {
  clickThenStableClick,
  stableBoundingBox,
} from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Self-contained source, one part, two notes per measure — enough to
 * distinguish "the anchor's own note" from "the range the second click
 * widens it into" without needing bar numbers or a system layout of its own.
 *
 *   Measure 0: "1 2" — note ids 0, 1
 *   Measure 1: "3 4" — note ids 2, 3
 *   Measure 2: "5 6" — note ids 4, 5
 */
const source = [
  '# metadata',
  'title = "pending second click test"',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0
  '',
  '[M] 3 4', // measure 1
  '',
  '[M] 5 6', // measure 2
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'pending-second-click-test.jianpu',
        userFiles: { 'pending-second-click-test.jianpu': source },
        bin: {},
        fileIds: {
          'pending-second-click-test.jianpu':
            'pending-second-click-test-id-001',
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
  await page.keyboard.type('12')
  await page.keyboard.press('Enter')
  await expect(page.locator('button.play-measure-btn')).toHaveText(/Measure/, {
    timeout: 5_000,
  })
}

// Every note renders two sibling `[data-tag="note"]` groups sharing the same
// `data-part-index`/`data-note-id` — one wrapping the (pointer-events: none)
// playback-cursor rect, the other wrapping the click-target rect (see
// `applyPersistedNoteHighlights`'s doc comment in `previewRangeHighlights.ts`)
// — so this can't just select the group by id and grab its rect: `:has()`
// picks out the click-target one specifically. `.first()` on top of that:
// the preview can transiently render a plain and a highlighted SVG document
// during the priming dance's async swap, so even the right group can briefly
// resolve to two elements — same rationale as other range-select specs' own
// `.first()`.
function noteClickTargetRect(
  page: import('@playwright/test').Page,
  noteId: number,
) {
  return page
    .locator(
      `[data-tag="note"][data-part-index="0"][data-note-id="${noteId}"]:has(rect[data-variant="note-click-target-rect"])`,
    )
    .locator('rect[data-variant="note-click-target-rect"]')
    .first()
}

Given('the pending-second-click fixture is loaded', async ({ page }) => {
  await loadFixture(page)
  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
  await page.waitForSelector('[data-tag="note"][data-note-id="0"]', {
    timeout: 10_000,
  })
  await primeMeasureSpans(page)
})

// Every color assertion below reads a note's click-target rect's own fill,
// but `preview.css`'s `g[data-tag="note"]:hover > rect[...]` hover rule
// paints a third color on top of both the pending and committed ones this
// spec cares about — so every step that leaves the mouse sitting over a note
// moves it away afterward, to a point past the last measure that hits
// nothing, keeping the assertions below reading the range-selected fill
// rather than the transient hover one.
async function moveMouseOffPreview(page: import('@playwright/test').Page) {
  await page.mouse.move(0, 0)
}

async function clickNote(
  page: import('@playwright/test').Page,
  noteId: number,
) {
  const rect = noteClickTargetRect(page, noteId)
  await expect(rect).toBeVisible({ timeout: 5_000 })
  const box = await stableBoundingBox(rect)
  if (!box) throw new Error(`Could not get a bounding box for note ${noteId}.`)
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.mouse.down()
  await page.mouse.up()
  await moveMouseOffPreview(page)
}

When('I click the first note in measure {int}', async ({ page }, measure) => {
  // Two notes per measure (see this file's own doc comment) — measure N's
  // first note is note id 2*N.
  await clickNote(page, measure * 2)
})

When(
  'I click-and-click select the first note in measure {int} then the first note in measure {int}',
  async ({ page }, fromMeasure, toMeasure) => {
    await clickThenStableClick(
      page,
      noteClickTargetRect(page, fromMeasure * 2),
      noteClickTargetRect(page, toMeasure * 2),
    )
    await moveMouseOffPreview(page)
  },
)

When('I click on empty space below the staff', async ({ page }) => {
  const previewPages = page.locator('.preview-pages')
  const box = await previewPages.boundingBox()
  if (!box) throw new Error('Could not get a bounding box for the preview.')
  // Land well below the rendered staff, inside the scrollable container but
  // off every recognizable click target — the click-and-click gesture's
  // "missed everything" cancel path (see `isEmptySpace`).
  await page.mouse.move(box.x + box.width / 2, box.y + box.height - 10)
  await page.mouse.down()
  await page.mouse.up()
})

// 'I press Escape' is defined once, shared across specs, in
// export-menu-ux.steps.ts.

const PENDING_FILL = 'rgba(217, 119, 6, 0.25)'
const COMMITTED_FILL = 'rgba(37, 99, 235, 0.25)'

Then(
  'that note is highlighted in the pending-selection color',
  async ({ page }) => {
    const rect = noteClickTargetRect(page, 0)
    await expect
      .poll(() => rect.evaluate((el) => getComputedStyle(el).fill), {
        timeout: 3_000,
      })
      .toBe(PENDING_FILL)
  },
)

Then(
  'that note is highlighted in the committed-selection color',
  async ({ page }) => {
    const rect = noteClickTargetRect(page, 0)
    await expect
      .poll(() => rect.evaluate((el) => getComputedStyle(el).fill), {
        timeout: 3_000,
      })
      .toBe(COMMITTED_FILL)
  },
)

Then(
  'the range-selected notes are highlighted in the committed-selection color',
  async ({ page }) => {
    const selected = page.locator('[data-tag="note"][data-note-range-selected]')
    await expect(selected.first()).toBeVisible()
    const fills = await selected
      .locator('rect[data-variant="note-click-target-rect"]')
      .evaluateAll((els) => els.map((el) => getComputedStyle(el).fill))
    for (const fill of fills) {
      expect(fill).toBe(COMMITTED_FILL)
    }
  },
)

Then('the pending-second-click banner is visible', async ({ page }) => {
  await expect(
    page.locator('[data-testid="pending-second-click-banner"]'),
  ).toBeVisible({ timeout: 3_000 })
})

Then('the pending-second-click banner is hidden', async ({ page }) => {
  await expect(
    page.locator('[data-testid="pending-second-click-banner"]'),
  ).toHaveCount(0, { timeout: 3_000 })
})
