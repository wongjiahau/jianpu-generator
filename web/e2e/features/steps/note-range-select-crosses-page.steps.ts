import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click note-range gesture not
 * spanning a page boundary (see `usePreviewClickSelection.ts`'s
 * `anchorStateRef`): the anchor from the first click is frozen as a raw
 * viewport-relative `(clientX, clientY)` point. Reaching a note on the next
 * page requires scrolling `.preview-pages` (it's the only element that
 * scrolls — see `preview.css`'s `.preview-pages { overflow: auto }`), and
 * that scroll invalidates the frozen anchor: the same screen coordinate now
 * sits over whatever scrolled into its place, not the originally-clicked
 * note. The second click's marquee is then built from a stale anchor point,
 * so it misses the intended range.
 *
 * Self-contained source with many single-measure systems (`break` on every
 * measure) so the score reliably overflows a single page without depending
 * on the default `max_measures_per_system` packing.
 */
const measures = Array.from({ length: 60 }, () => '[M] 1 2 3 4').join('\n\n')

const crossPageSource = [
  '# metadata',
  'title = "cross page range test"',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '',
  measures,
].join('\n')

function pages(page: import('@playwright/test').Page) {
  return page.locator('.preview-page')
}

// A note renders two sibling `[data-tag="note"]` groups (the click-target
// group and the pointer-events-none playback-cursor group — see
// `applyPersistedNoteHighlights`'s doc comment) sharing the same
// `data-note-id`/`data-part-index`, so this narrows to the one that actually
// carries the click-target rect and gets the `noteRangeSelected` flag.
function notesOnPage(page: import('@playwright/test').Page, pageIndex: number) {
  return pages(page)
    .nth(pageIndex)
    .locator(
      '[data-tag="note"]:has(rect[data-variant="note-click-target-rect"])',
    )
}

Given(
  'the cross-page range-selection fixture is loaded and note click targets have rendered',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'cross-page-range-test.jianpu',
          userFiles: { 'cross-page-range-test.jianpu': source },
          bin: {},
          fileIds: {
            'cross-page-range-test.jianpu': 'cross-page-range-test-id-001',
          },
        }),
      )
    }, crossPageSource)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
      timeout: 10_000,
    })
    await expect
      .poll(async () => pages(page).count(), { timeout: 15_000 })
      .toBeGreaterThanOrEqual(2)
  },
)

When(
  'I click-and-click select the first note on page 1 then the third note on page 2, scrolling the second note into view first',
  async ({ page }) => {
    const firstNote = notesOnPage(page, 0).first()
    const firstBox = await stableBoundingBox(firstNote)
    if (!firstBox)
      throw new Error(
        'Could not get bounding box for the first note on page 1.',
      )

    // Click #1 — anchors the gesture at the first note's screen position.
    await page.mouse.move(
      firstBox.x + firstBox.width / 2,
      firstBox.y + firstBox.height / 2,
    )
    await page.mouse.down()
    await page.mouse.up()

    // Reaching a note on page 2 requires scrolling — this is the step that
    // invalidates the frozen anchor point (see this file's doc comment).
    const targetNote = notesOnPage(page, 1).nth(2)
    await targetNote.scrollIntoViewIfNeeded()
    const targetBox = await stableBoundingBox(targetNote)
    if (!targetBox)
      throw new Error(
        'Could not get bounding box for the third note on page 2.',
      )

    await page.mouse.move(
      targetBox.x + targetBox.width / 2,
      targetBox.y + targetBox.height / 2,
      {
        steps: 10,
      },
    )
    await page.mouse.down()
    await page.mouse.up() // click #2 — commits
  },
)

Then('the first note on page 1 is still range-selected', async ({ page }) => {
  await expect(notesOnPage(page, 0).first()).toHaveAttribute(
    'data-note-range-selected',
    '',
  )
})

Then('the third note on page 2 is range-selected', async ({ page }) => {
  await expect(notesOnPage(page, 1).nth(2)).toHaveAttribute(
    'data-note-range-selected',
    '',
  )
})
