import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click note-range gesture not
 * spanning a system boundary (see `previewRangeHighlights.ts`'s
 * `cellsInMarquee`): the gesture resolves to a plain axis-aligned rectangle
 * between the anchor and the second click's screen point, rather than "every
 * note in reading order between the two clicks" — so an anchor/target pair
 * that both sit near the same x-column (the left edge of their own system)
 * produces a narrow marquee that captures only those two endpoint columns,
 * missing the notes in between (indices 1-3, which sit further right in
 * measure 0's row), even though both systems are fully visible with no
 * scrolling involved.
 *
 * `break` forces measure 1 onto its own system regardless of
 * `max_measures_per_system`, so this fixture needs no filler content to
 * reproduce the boundary.
 */
const crossSystemSource = [
  '# metadata',
  'title = "cross system range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — system 1 — notes 0-3
  '',
  'break',
  '[M] 5 6 7 1', // measure 1 — system 2 — notes 4-7
].join('\n')

function noteRects(page: import('@playwright/test').Page) {
  return page.locator('rect[data-variant="note-click-target-rect"]')
}

// A note renders two sibling `[data-tag="note"]` groups (the click-target
// group and the pointer-events-none playback-cursor group — see
// `applyPersistedNoteHighlights`'s doc comment) sharing the same
// `data-note-id`/`data-part-index`, so this narrows to the one that actually
// carries the click-target rect and gets the `noteRangeSelected` flag.
function noteGroup(page: import('@playwright/test').Page, noteId: number) {
  return page.locator(
    `[data-tag="note"][data-note-id="${noteId}"]:has(rect[data-variant="note-click-target-rect"])`,
  )
}

Given(
  'the cross-system range-selection fixture is loaded and note click targets have rendered',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'cross-system-range-test.jianpu',
          userFiles: { 'cross-system-range-test.jianpu': source },
          bin: {},
          fileIds: {
            'cross-system-range-test.jianpu': 'cross-system-range-test-id-001',
          },
        }),
      )
    }, crossSystemSource)
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

When(
  'I click-and-click select the note at index {int} then the note at index {int}',
  async ({ page }, from: number, to: number) => {
    const box0 = await stableBoundingBox(noteRects(page).nth(from))
    if (!box0) {
      throw new Error(`Could not get bounding box for note ${from}.`)
    }
    await page.mouse.move(box0.x + box0.width / 2, box0.y + box0.height / 2)
    await page.mouse.down()
    await page.mouse.up() // click #1 — anchors

    // The anchoring click self-commits into a Monaco selection, whose
    // cursor-change listener debounces (300ms) into a worker round-trip that
    // swaps in fresh "highlighted documents" SVG DOM (see
    // `note-range-select-highlight.steps.ts`'s doc comment) — wait for that to
    // settle before re-querying the target note's rect, so its bounding box
    // isn't captured mid-swap.
    await page.waitForTimeout(400)

    const box1 = await stableBoundingBox(noteRects(page).nth(to))
    if (!box1) {
      throw new Error(`Could not get bounding box for note ${to}.`)
    }
    await page.mouse.move(box1.x + box1.width / 2, box1.y + box1.height / 2, {
      steps: 10,
    })
    await page.mouse.down()
    await page.mouse.up() // click #2 — commits
  },
)

Then(
  'the notes at index {int}, {int}, {int}, {int} and {int} are all range-selected',
  async ({ page }, a: number, b: number, c: number, d: number, e: number) => {
    for (const noteId of [a, b, c, d, e]) {
      await expect(noteGroup(page, noteId)).toHaveAttribute(
        'data-note-range-selected',
        '',
      )
    }
  },
)
