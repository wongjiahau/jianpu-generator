import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

// Lines (1-based):
//    7: # sequence
//    8: A, B, C, A
//   11: time=4/4 key=C4 bpm=120 label="A"  ← view-zone directive
//   12: 1 2 3 4                             ← measure 0 ("A")
//   14: label="B"                           ← view-zone directive
//   15: 5 6 7 1'                            ← measure 1 ("B")
//   17: label="C"                           ← view-zone directive
//   18: 1' 7 6 5                            ← measure 2 ("C")
//
// The sequence's second "A" is a repeat — it reuses measure 0's written
// lines rather than duplicating them, so the document has only 3 written
// measures for 4 sequence entries.
const SOURCE = [
  '# metadata',
  'title = "test"',
  '',
  '# parts',
  'M = notes',
  '',
  '# sequence',
  'A, B, C, A',
  '',
  '# score',
  'time=4/4 key=C4 bpm=120 label="A"',
  '1 2 3 4',
  '',
  'label="B"',
  "5 6 7 1'",
  '',
  'label="C"',
  "1' 7 6 5",
].join('\n')

async function getEditorSelections(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const monacoApi = (
      window as unknown as { monaco: typeof import('monaco-editor') }
    ).monaco
    const selections = monacoApi.editor.getEditors()[0]?.getSelections() ?? []
    return selections.map((s) => ({
      startLineNumber: s.startLineNumber,
      endLineNumber: s.endLineNumber,
    }))
  })
}

// Both toolbars render `button.section-jump-btn` inside their own
// `[role="toolbar"]`; SequenceJumpToolbar mounts second.
function sequenceToolbarButtons(page: import('@playwright/test').Page) {
  return page
    .locator('[role="toolbar"]')
    .nth(1)
    .locator('button.section-jump-btn')
}

Given(
  'a sequence chain {string} over sections {string}, {string}, {string} is seeded for chain highlight',
  async (
    { page },
    _chain: string,
    _sectionA: string,
    _sectionB: string,
    _sectionC: string,
  ) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'sequence-chain-highlight-test.jianpu',
          userFiles: { 'sequence-chain-highlight-test.jianpu': src },
          bin: {},
          fileIds: {
            'sequence-chain-highlight-test.jianpu': crypto.randomUUID(),
          },
        }),
      )
    }, SOURCE)
  },
)

When(
  'the app loads with the chain-highlight sequence toolbar ready',
  async ({ page }) => {
    await page.goto('/')
    // Four sequence entries: "A", "B", "C", "A" (the repeat).
    await expect(sequenceToolbarButtons(page)).toHaveCount(4, {
      timeout: 15_000,
    })
  },
)

When(
  'I drag from the {string} sequence entry to the repeated {string} sequence entry',
  async ({ page }, _from: string, _to: string) => {
    const buttons = sequenceToolbarButtons(page)

    // Drag from the "C" button (index 2) to the second "A" button (index 3,
    // the repeat).
    await buttons.nth(2).hover()
    await page.mouse.down()
    await buttons.nth(3).hover()
    await page.mouse.up()
  },
)

Then(
  'the editor selects exactly {string} and the repeated {string} as disjoint ranges',
  async ({ page }, _first: string, _second: string) => {
    await expect
      .poll(() => getEditorSelections(page), { timeout: 3_000 })
      .toEqual([
        { startLineNumber: 17, endLineNumber: 18 }, // "C" only
        { startLineNumber: 11, endLineNumber: 12 }, // "A" only (the repeat resolves to the same written lines)
      ])
  },
)

Then(
  'the preview highlights exactly the {string} and {string} measures, excluding {string}',
  async ({ page }, _first: string, _second: string, _excluded: string) => {
    const highlightRects = page.locator(
      '.preview-page [data-testid="measure-highlight"]',
    )
    await expect(highlightRects).toHaveCount(2, { timeout: 3_000 })

    const measureBox = async (measureIndex: number) =>
      stableBoundingBox(
        page
          .locator(`[data-tag="measure"][data-measure-index="${measureIndex}"]`)
          .first(),
      )

    const cBox = await measureBox(2)
    const aBox = await measureBox(0)
    const bBox = await measureBox(1)
    if (!cBox || !aBox || !bBox) {
      throw new Error('Could not get bounding boxes for the measures.')
    }

    const highlightBoxes = await highlightRects.evaluateAll((rects) =>
      rects.map((r) => r.getBoundingClientRect()),
    )

    const matchesBox = (
      highlight: { x: number; y: number },
      target: { x: number; y: number },
    ) =>
      Math.abs(highlight.x - target.x) < 5 &&
      Math.abs(highlight.y - target.y) < 5

    expect(highlightBoxes.some((h) => matchesBox(h, cBox))).toBe(true)
    expect(highlightBoxes.some((h) => matchesBox(h, aBox))).toBe(true)
    expect(highlightBoxes.some((h) => matchesBox(h, bBox))).toBe(false)
  },
)
