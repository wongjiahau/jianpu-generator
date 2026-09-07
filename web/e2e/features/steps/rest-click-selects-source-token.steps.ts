import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { focusEditor } from '../../fileSwitcherHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression test: clicking a rest ("0") in the SVG preview must select the
 * "0" in the Monaco editor, the same as clicking any other note does (see
 * `note_spans.rs`'s `list_note_spans_from_source` — a rest's `NoteSourceSpan`
 * used to have no byte range at all, so a selection made up of just a rest
 * cell was silently dropped instead of reaching Monaco).
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" and a single rest among plain notes, so all four note click
 * targets render side by side in one row.
 */
const restClickTestSource = [
  '# metadata',
  'title = "rest click test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 0 3 4', // measure 0 — line 9
].join('\n')

async function getEditorSelectedText(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const monacoApi = (
      window as unknown as { monaco: typeof import('monaco-editor') }
    ).monaco
    const editor = monacoApi.editor.getEditors()[0]
    const selection = editor?.getSelection()
    const model = editor?.getModel()
    if (!selection || !model) return null
    return model.getValueInRange(selection)
  })
}

Given('the rest-click test fixture is loaded', async ({ page }) => {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'rest-click-test.jianpu',
        userFiles: { 'rest-click-test.jianpu': source },
        bin: {},
        fileIds: { 'rest-click-test.jianpu': 'rest-click-test-id-001' },
      }),
    )
  }, restClickTestSource)

  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
  await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
    timeout: 10_000,
  })
  const noteRects = page.locator('rect[data-variant="note-click-target-rect"]')
  await expect(noteRects).toHaveCount(4, { timeout: 10_000 })

  // Prime the editor/worker round-trip the same way the other click-select
  // specs do (see `measure-click-selects-notes.spec.ts`'s
  // `primeMeasureSpans`), so measureSpans/noteSpans are settled before
  // hit-testing. Also wait for the resulting highlight re-render to finish
  // swapping the SVG DOM, otherwise a bounding box captured mid-swap can be
  // stale by the time the click-and-click gesture below runs.
  await focusEditor(page)
  await page.keyboard.press('Control+g')
  await page.keyboard.type('9')
  await page.keyboard.press('Enter')
  await expect(page.locator('button.play-measure-btn')).toHaveText(/Measure/, {
    timeout: 5_000,
  })
  await expect(
    page.locator('.preview-page [data-testid="measure-highlight"]').first(),
  ).toBeVisible({ timeout: 5_000 })
})

When(
  "I click-and-click just past the note-range-select arm threshold inside the rest's own click target",
  async ({ page }) => {
    const noteRects = page.locator(
      'rect[data-variant="note-click-target-rect"]',
    )
    // Note index 1 is the rest ("0" in "1 0 3 4").
    const restBox = await stableBoundingBox(noteRects.nth(1))
    if (!restBox) {
      throw new Error(
        'Could not get bounding box for the rest note. ' +
          'Ensure the SVG preview has rendered.',
      )
    }

    // A single click already selects just the rest itself (see
    // `previewClickHandler.ts`'s 'note' mode) — this clicks twice instead,
    // staying inside the rest's own click target both times, to exercise the
    // 'note' marquee-resolution path specifically rather than the
    // first-click self-commit path.
    await clickAndClickSelect(
      page,
      restBox.x + 2,
      restBox.y + restBox.height / 2,
      restBox.x + restBox.width - 2,
      restBox.y + restBox.height / 2,
      5,
    )
  },
)

Then(
  '{int} note is range-selected, as seen in rest click selects source token',
  async ({ page }, count: number) => {
    const highlightedNotes = page.locator(
      '[data-tag="note"][data-note-range-selected]',
    )
    await expect(highlightedNotes).toHaveCount(count)
  },
)

Then(
  'the play-measure button reads Selection, as seen in rest click selects source token',
  async ({ page }) => {
    // The repurposed play-measure button switching to "▶ Selection" confirms
    // the rest's byte range was pushed into Monaco/App state, not dropped.
    await expect(page.locator('button.play-measure-btn')).toHaveText(
      /Selection/,
      { timeout: 3_000 },
    )
  },
)

Then(
  "the editor's selected text is {string}",
  async ({ page }, expected: string) => {
    await expect(async () => {
      expect(await getEditorSelectedText(page)).toBe(expected)
    }).toPass({ timeout: 3_000 })
  },
)
