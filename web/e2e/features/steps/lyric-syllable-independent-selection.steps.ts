import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Each lyric syllable gets its own `Tag::Lyric` click target
 * (`data-tag="lyric"`, `data-part-index`/`data-note-id`/`data-verse`), kept
 * independent of the note-selection stack (`Tag::Note`,
 * `[data-tag="note"][data-note-range-selected]`) for a *syllable-level*
 * click / click-and-click — see `useLyricSelection.ts` and `Preview.tsx`'s
 * `onLyricRangeSelect`/`selectedLyricCells`. This spec covers the
 * cross-cutting independence matrix: a syllable-level lyric click /
 * click-and-click never touches note selection, a lyric range selection's
 * Monaco selection matches the selected source text, and separate verse rows
 * select independently — except a *measure*-level click / click-and-click
 * (on a note or the space around it), which is a shortcut that intentionally
 * selects both notes and every verse's lyrics in that measure at once (see
 * `Preview.tsx`'s `onMeasureRangeSelect`).
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system", one measure of four single-beat notes, and two verses so both
 * single-verse and multi-verse independence can be exercised from one
 * fixture.
 */
const multiVerseSource = [
  '# metadata',
  'title = "lyric independent selection test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — line 9
  'do re mi fa', // verse 0 — line 10
  'uno dos tres cuatro', // verse 1 — line 11
].join('\n')

function lyricRect(
  page: import('@playwright/test').Page,
  noteId: number,
  verse: number,
) {
  return page
    .locator(
      `[data-tag="lyric"][data-note-id="${noteId}"][data-verse="${verse}"]`,
    )
    .locator('rect')
}

async function monacoSelectionText(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const ed = window.monaco.editor.getEditors()[0]
    const model = ed.getModel()
    return model?.getValueInRange(ed.getSelection())
  })
}

Given(
  'the multi-verse lyric independence fixture is loaded and both verses have rendered',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'lyric-independent-test.jianpu',
          userFiles: { 'lyric-independent-test.jianpu': source },
          bin: {},
          fileIds: {
            'lyric-independent-test.jianpu': 'lyric-independent-test-id-001',
          },
        }),
      )
    }, multiVerseSource)
    await page.goto('/')
    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
      timeout: 10_000,
    })
    // Wait for both verses' click targets (4 syllables x 2 verses) to render.
    await expect(page.locator('[data-tag="lyric"]')).toHaveCount(8, {
      timeout: 10_000,
    })
    // Let layout fully settle before any bounding-box reads below — under
    // parallel test-suite load, the count above can pass a beat before the
    // two-verse layout has stopped shifting.
    await page.waitForTimeout(200)
  },
)

When(
  'I click syllable {int} of verse {int} without clicking a second time',
  async ({ page }, noteId: number, verse: number) => {
    const box = await stableBoundingBox(lyricRect(page, noteId, verse))
    if (!box) throw new Error('no box')
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await page.mouse.down()
    await page.waitForTimeout(50)
    await page.mouse.up()
    await page.waitForTimeout(100)
  },
)

When(
  'I click syllable {int} then click syllable {int} of verse {int}',
  async ({ page }, fromNoteId: number, toNoteId: number, verse: number) => {
    const start = await stableBoundingBox(lyricRect(page, fromNoteId, verse))
    const end = await stableBoundingBox(lyricRect(page, toNoteId, verse))
    if (!start || !end) throw new Error('no box')

    await clickAndClickSelect(
      page,
      start.x + start.width / 2,
      start.y + start.height / 2,
      end.x + end.width / 2,
      end.y + end.height / 2,
    )
    await page.waitForTimeout(200)
  },
)

When(
  "I click near the top of note {int}'s click target without clicking a second time",
  async ({ page }, noteId: number) => {
    // The note's click target row is widened to also cover both verse rows
    // beneath it (see `part_row_ranges`), so its vertical center can land
    // inside a lyric row once there's more than one verse. Click near the top
    // of the rect instead — solidly inside the note glyph's own zone, above
    // where the lyric rows start.
    const noteRect = page
      .locator(`[data-tag="note"][data-note-id="${noteId}"]`)
      .locator('rect[data-variant="note-click-target-rect"]')
    const box = await stableBoundingBox(noteRect)
    if (!box) throw new Error('no box')
    await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.15)
    await page.mouse.down()
    await page.waitForTimeout(50)
    await page.mouse.up()
    await page.waitForTimeout(100)
  },
)

When(
  "I Ctrl-click near the top of note {int}'s click target",
  async ({ page }, noteId: number) => {
    // The note's click target row is widened to also cover both verse rows
    // beneath it (see `part_row_ranges`), so its vertical center can land
    // inside a lyric row once there's more than one verse. Click near the top
    // of the rect instead — solidly inside the note glyph's own zone, above
    // where the lyric rows start.
    const noteRect = page
      .locator(`[data-tag="note"][data-note-id="${noteId}"]`)
      .locator('rect[data-variant="note-click-target-rect"]')
    const box = await stableBoundingBox(noteRect)
    if (!box) throw new Error('no box')
    await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.15)
    await page.keyboard.down('Control')
    await page.mouse.down()
    await page.waitForTimeout(50)
    await page.mouse.up()
    await page.keyboard.up('Control')
    await page.waitForTimeout(100)
  },
)

Then(
  'only syllable {int} of verse {int} is range-selected',
  async ({ page }, noteId: number, verse: number) => {
    await expect(
      page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
    ).toHaveCount(1)
    await expect(
      page.locator(
        `[data-tag="lyric"][data-lyric-range-selected][data-note-id="${noteId}"][data-verse="${verse}"]`,
      ),
    ).toHaveCount(1)
  },
)

Then(
  'no note is range-selected by the syllable-level interaction',
  async ({ page }) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(0)
  },
)

Then('no lyric syllable is range-selected', async ({ page }) => {
  await expect(
    page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
  ).toHaveCount(0)
})

Then(
  '{int} lyric syllables in total are range-selected',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="lyric"][data-lyric-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  'exactly {int} note is range-selected via the note click target',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  'exactly {int} notes are range-selected via the note click target',
  async ({ page }, count: number) => {
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  'the Monaco selection text is {string}',
  async ({ page }, text: string) => {
    const selectedText = await monacoSelectionText(page)
    expect(selectedText).toBe(text)
  },
)

Then(
  'syllable {int} of verse {int} is range-selected but syllable {int} of verse {int} is not',
  async (
    { page },
    noteIdA: number,
    verseA: number,
    noteIdB: number,
    verseB: number,
  ) => {
    await expect(
      page.locator(
        `[data-tag="lyric"][data-lyric-range-selected][data-note-id="${noteIdA}"][data-verse="${verseA}"]`,
      ),
    ).toHaveCount(1)
    // Verse 0's corresponding syllable must not also be marked selected.
    await expect(
      page.locator(
        `[data-tag="lyric"][data-lyric-range-selected][data-note-id="${noteIdB}"][data-verse="${verseB}"]`,
      ),
    ).toHaveCount(0)
  },
)
