import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * A lyric syllable has its own `Tag::Lyric` click-target rect
 * (`data-tag="lyric"`, see `render_lyric_click_target` in
 * `src/renderer/new_renderer.rs`), sized to its own column and resolved
 * after — so painted on top of, for `elementFromPoint` hit-testing purposes
 * — the wider `NoteClickTarget` rect that also covers that note's lyric row
 * (see `resolve_click_target_elements` in `src/coordinate_resolver/resolve.rs`).
 * A click landing on or near the lyric glyph's ink therefore resolves to the
 * lyric's own selection, independent of the note it belongs to — see
 * `lyric-syllable-independent-selection.feature` for the fuller independence
 * matrix (note-click vs. lyric-click, multi-verse, Monaco sync).
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" and four single-beat notes with one syllable each, so all four
 * note/lyric pairs render side by side in one row and stay within the
 * viewport during the range-select.
 */
const rangeTestSource = [
  '# metadata',
  'title = "lyric range-select test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — line 9
  'do re mi fa', // line 10
].join('\n')

function lyricTexts(page: import('@playwright/test').Page) {
  // Lyric syllables are the only text glyphs tagged with the "lyric" data
  // variant (see `render_lyric`), so this selector picks them out reliably
  // regardless of their actual text content.
  return page.locator('svg text[data-variant="lyric"]')
}

Given(
  'the lyric range-select test fixture is loaded and the first measure has rendered',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'lyric-range-test.jianpu',
          userFiles: { 'lyric-range-test.jianpu': source },
          bin: {},
          fileIds: { 'lyric-range-test.jianpu': 'lyric-range-test-id-001' },
        }),
      )
    }, rangeTestSource)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
      timeout: 10_000,
    })

    await expect(lyricTexts(page)).toHaveCount(4, { timeout: 10_000 })
  },
)

When(
  'I click-and-click select from lyric syllable {int} to lyric syllable {int}',
  async ({ page }, from: number, to: number) => {
    const boxFrom = await stableBoundingBox(lyricTexts(page).nth(from))
    const boxTo = await stableBoundingBox(lyricTexts(page).nth(to))
    if (!boxFrom || !boxTo) {
      throw new Error(
        `Could not get bounding boxes for lyric syllables ${from} and ${to}.`,
      )
    }

    const startX = boxFrom.x + boxFrom.width / 2
    const startY = boxFrom.y + boxFrom.height / 2
    const endX = boxTo.x + boxTo.width / 2
    const endY = boxTo.y + boxTo.height / 2

    // Click-and-click a range across the syllables.
    await clickAndClickSelect(page, startX, startY, endX, endY)
  },
)

When(
  'I click lyric syllable {int} once',
  async ({ page }, index: number) => {
    // A plain click (mousedown + mouseup at the same point, not a
    // click-and-click range) selects just this syllable.
    const box = await stableBoundingBox(lyricTexts(page).nth(index))
    if (!box) throw new Error('no box')
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await page.mouse.down()
    await page.waitForTimeout(50)
    await page.mouse.up()
  },
)

Then(
  'lyric syllables {int}, {int} and {int} are range-selected',
  async ({ page }, a: number, b: number, c: number) => {
    // The range-select resolves through the syllables' own lyric click
    // targets — lyric selection is independent of note selection, so no
    // note cell gets highlighted by this range-select at all.
    const highlightedLyrics = page.locator(
      '[data-tag="lyric"][data-lyric-range-selected]',
    )
    await expect(highlightedLyrics).toHaveCount(3)
    for (const noteId of [a, b, c]) {
      await expect(
        page.locator(
          `[data-tag="lyric"][data-lyric-range-selected][data-note-id="${noteId}"]`,
        ),
      ).toHaveCount(1)
    }
  },
)

Then(
  'only lyric syllable {int} is range-selected',
  async ({ page }, noteId: number) => {
    await expect(
      page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
    ).toHaveCount(1)
    await expect(
      page.locator(
        `[data-tag="lyric"][data-lyric-range-selected][data-note-id="${noteId}"]`,
      ),
    ).toHaveCount(1)
  },
)

Then('no note is range-selected', async ({ page }) => {
  await expect(
    page.locator('[data-tag="note"][data-note-range-selected]'),
  ).toHaveCount(0)
})
