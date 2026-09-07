import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click range gesture's *second* click
 * re-scrolling the preview after it commits (see `previewClickHandler.ts`'s
 * `handleCommitClick` and `Preview.tsx`'s scroll-to-selection effect): the
 * commit pushes a Monaco selection, which — after `notifySelection`'s
 * debounce — lands back in `Preview.tsx`'s `selectedMeasureRange` effect and
 * `scrollIntoView({ block: 'center' })`s the target measure, silently
 * undoing whatever scroll position the user's own `scrollIntoViewIfNeeded`
 * (needed to reach the second click's target at all) left the preview at.
 *
 * Many single-measure systems (`break` on every measure) so the score
 * reliably overflows the preview's own scrollable height without depending
 * on the default `max_measures_per_system` packing.
 */
const measures = Array.from({ length: 60 }, () => '[M] 1 2 3 4').join('\n\n')

const scrollPreservingSource = [
  '# metadata',
  'title = "scroll preserving range test"',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '',
  measures,
].join('\n')

function previewPages(page: import('@playwright/test').Page) {
  return page.locator('.preview-pages')
}

function notes(page: import('@playwright/test').Page) {
  return page.locator(
    '[data-tag="note"]:has(rect[data-variant="note-click-target-rect"])',
  )
}

Given(
  'the scroll-preserving range-selection fixture is loaded and note click targets have rendered',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'scroll-preserving-range-test.jianpu',
          userFiles: { 'scroll-preserving-range-test.jianpu': source },
          bin: {},
          fileIds: {
            'scroll-preserving-range-test.jianpu':
              'scroll-preserving-range-test-id-001',
          },
        }),
      )
    }, scrollPreservingSource)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
      timeout: 10_000,
    })
  },
)

let scrollTopAfterManualScroll = -1

When(
  'I click-and-click select the first note then scroll a far-away note into view and click it',
  async ({ page }) => {
    const firstNote = notes(page).first()
    const firstBox = await stableBoundingBox(firstNote)
    if (!firstBox)
      throw new Error('Could not get bounding box for the first note.')

    // Click #1 — anchors the gesture at the first note's screen position.
    await page.mouse.move(
      firstBox.x + firstBox.width / 2,
      firstBox.y + firstBox.height / 2,
    )
    await page.mouse.down()
    await page.mouse.up()

    // Reaching a note far down the score requires scrolling — this is the
    // user's own, deliberate scroll that the bug then fights.
    const targetNote = notes(page).nth(200)
    await targetNote.scrollIntoViewIfNeeded()
    const targetBox = await stableBoundingBox(targetNote)
    if (!targetBox)
      throw new Error('Could not get bounding box for the far-away note.')

    scrollTopAfterManualScroll = await previewPages(page).evaluate(
      (el) => el.scrollTop,
    )

    await page.mouse.move(
      targetBox.x + targetBox.width / 2,
      targetBox.y + targetBox.height / 2,
      { steps: 10 },
    )
    await page.mouse.down()
    await page.mouse.up() // click #2 — commits
  },
)

Then(
  "the preview's scroll position is unchanged from right after the manual scroll",
  async ({ page }) => {
    // Sample repeatedly instead of a single poll-to-match: the bug's
    // re-scroll fires and then settles there, and a plain `.poll().toBe()`
    // would only ever notice a value that never converges — recording every
    // observed value catches a transient re-scroll too. 1.5s covers the
    // commit's own debounced `notifySelection` plus the async
    // `renderWithHighlightRange` worker round-trip that follows it.
    const observed: number[] = []
    for (let i = 0; i < 15; i++) {
      observed.push(await previewPages(page).evaluate((el) => el.scrollTop))
      await new Promise((resolve) => setTimeout(resolve, 100))
    }
    expect(
      observed.every((value) => value === scrollTopAfterManualScroll),
    ).toBe(true)
  },
)
