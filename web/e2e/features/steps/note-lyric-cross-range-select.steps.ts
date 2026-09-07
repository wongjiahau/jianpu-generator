import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression test for a range-select that starts on one cell type (note
 * or lyric) and whose second click lands across the other type never
 * selecting that other type at all.
 *
 * `PreviewAnchorState` (see `previewAnchorState.ts`) is a discriminated union
 * that commits to exactly one mode on the anchoring click — a click that
 * lands on a note anchors `'note'` mode, and a click that lands on a lyric
 * syllable anchors `'lyric'` mode. For the rest of the gesture,
 * `previewClickHandler.ts`'s resolution calls only that one mode's
 * highlighter — `applyNoteRangeHighlights` for `'note'` mode,
 * `applyLyricRangeHighlights` for `'lyric'` mode — never both.
 *
 * That's a real behavioral gap: a range-select that starts on a NOTE and
 * whose second click lands below it, so it visually covers the lyric
 * syllables underneath, does NOT select those syllables. The symmetric
 * case — starting on a LYRIC syllable with the second click above it over
 * the notes — does not select those notes either.
 *
 * Contrast with `'measure'` mode (a gesture starting on empty space or a
 * bare bar line), which unions both: its resolution calls
 * `noteCellsInMeasureRange` and `lyricCellsInMeasureRange` together and
 * applies both highlight sets (see `previewClickHandler.ts`).
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" and four single-beat notes with one syllable each, so all four
 * note/lyric pairs render side by side in one row and stay within the
 * viewport during the range-select — same fixture shape as
 * `lyric-range-select-highlight.feature`.
 */
const rangeTestSource = [
  '# metadata',
  'title = "note lyric cross range-select test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — line 9
  'do re mi fa', // verse 0 — line 10
].join('\n')

function noteRects(page: import('@playwright/test').Page) {
  return page.locator('rect[data-variant="note-click-target-rect"]')
}

function lyricTexts(page: import('@playwright/test').Page) {
  // Lyric syllables are the only text glyphs tagged with the "lyric" data
  // variant (see `render_lyric`), so this selector picks them out reliably
  // regardless of their actual text content.
  return page.locator('svg text[data-variant="lyric"]')
}

Given(
  'the note-lyric cross range-select test fixture is loaded and both rows have rendered',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'note-lyric-cross-range-test.jianpu',
          userFiles: { 'note-lyric-cross-range-test.jianpu': source },
          bin: {},
          fileIds: {
            'note-lyric-cross-range-test.jianpu':
              'note-lyric-cross-range-test-id-001',
          },
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

    await expect(noteRects(page)).toHaveCount(4, { timeout: 10_000 })
    await expect(lyricTexts(page)).toHaveCount(4, { timeout: 10_000 })
    // Let layout fully settle before reading any bounding boxes below — this
    // range-select crosses the note/lyric row boundary, so it's sensitive to
    // exactly where that boundary lands, unlike a same-row range-select. A
    // fixed timeout alone (this file's original 200ms) is not reliable here:
    // web-font metrics finishing load can reflow the note/lyric rows'
    // vertical position well after the note/lyric *counts* above are already
    // satisfied, shifting a row by tens of pixels and making the range-select
    // miss its intended note/lyric entirely — waiting on `document.fonts.ready`
    // first closes that window; the short timeout after it is now just a
    // final safety margin for any post-font reflow (e.g. layout-affecting
    // React state settling), same fixture-settle pattern as
    // `lyric-syllable-independent-selection.feature`'s Background.
    await page.evaluate(() => document.fonts.ready)
    await page.waitForTimeout(200)
  },
)

When(
  "I click-and-click select from note {int}'s click target down and across to lyric syllable {int}",
  async ({ page }, noteIndex: number, lyricIndex: number) => {
    // Anchor the range-select on note 0's own click-target rect, and click
    // down/across to lyric syllable 2 ("mi") — a range that visually spans
    // notes 0-2 and their lyric row underneath.
    const noteBox0 = await stableBoundingBox(noteRects(page).nth(noteIndex))
    const lyricBox2 = await stableBoundingBox(lyricTexts(page).nth(lyricIndex))
    if (!noteBox0 || !lyricBox2) {
      throw new Error(
        `Could not get bounding boxes for note ${noteIndex} and lyric syllable ${lyricIndex}.`,
      )
    }

    const startX = noteBox0.x + noteBox0.width / 2
    const startY = noteBox0.y + noteBox0.height / 2
    const endX = lyricBox2.x + lyricBox2.width / 2
    const endY = lyricBox2.y + lyricBox2.height / 2

    await clickAndClickSelect(page, startX, startY, endX, endY)
  },
)

When(
  "I click-and-click select from lyric syllable {int} up and across to note {int}'s click target",
  async ({ page }, lyricIndex: number, noteIndex: number) => {
    // Anchor the range-select on lyric syllable 0 ("do"), and click up/across
    // to note 2's click-target rect — the symmetric case, a range that
    // visually spans the lyric row and notes 0-2 above it.
    const lyricBox0 = await stableBoundingBox(lyricTexts(page).nth(lyricIndex))
    const noteBox2 = await stableBoundingBox(noteRects(page).nth(noteIndex))
    if (!lyricBox0 || !noteBox2) {
      throw new Error(
        `Could not get bounding boxes for lyric syllable ${lyricIndex} and note ${noteIndex}.`,
      )
    }

    const startX = lyricBox0.x + lyricBox0.width / 2
    const startY = lyricBox0.y + lyricBox0.height / 2
    const endX = noteBox2.x + noteBox2.width / 2
    const endY = noteBox2.y + noteBox2.height / 2

    await clickAndClickSelect(page, startX, startY, endX, endY)
  },
)

Then(
  '{int} notes are range-selected by the cross-row range',
  async ({ page }, count: number) => {
    // The range-select started on a note, so note mode is armed and notes
    // 0-2 get selected as expected. (Symmetric case: notes selected via the
    // lyric-mode cross-row bug being asserted against.)
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  '{int} lyric syllables are range-selected by the cross-row range',
  async ({ page }, count: number) => {
    // Bug: the range also visually covers the other row's cells, but since
    // the range-select is locked into a single mode, the other cell type
    // never gets marked as range-selected.
    await expect(
      page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
    ).toHaveCount(count)
  },
)
