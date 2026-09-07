import { expect } from '@playwright/test'
import { DATA_VARIANT } from '../../../src/dataVariant'
import { Given, When } from './fixtures'

const SINGLE_MEASURE_SOURCE = [
  '# metadata',
  'title = "Test"',
  '',
  '# parts',
  'Melody = notes',
  'Bass = notes',
  '',
  '# score',
  '(bpm=120 key=C4 time=4/4)',
  '[Melody] 1 2 3 4',
  '[Bass] 5 6 7 1',
].join('\n')

// A tied note whose continuation is written as a bare repeat atom (`_`),
// which repeats the previous note's pitch *and* octave with no digit or
// `'`/`,` marker of its own in the source — see `parse_repeat_unit`'s doc
// comment in the Rust parser. Regression coverage for the "octave up then
// down doesn't restore a tied note" bug.
const SINGLE_MEASURE_TIED_REPEAT_NOTE_SOURCE = [
  '# metadata',
  'title = "Test"',
  '',
  '# parts',
  'Melody = notes',
  '',
  '# score',
  '(bpm=120 key=C4 time=4/4)',
  '[Melody] 1~ _ 5 5',
].join('\n')

const TWO_MEASURE_SOURCE = [
  '# metadata',
  'title = "Test"',
  '',
  '# parts',
  'Melody = notes',
  '',
  '# score',
  '(bpm=120 key=C4 time=4/4)',
  '[Melody] 1 2 3 4',
  '',
  '[Melody] 5 6 7 1',
].join('\n')

// Same shape as `part-label-click-selects-notes.steps.ts`'s two-part
// click-test fixture (short `[M]`/`[H]` labels, generous
// `max_measures_per_system` so both measures land in one system, part index
// 0 is Melody) so the globally registered "I plain-click the Melody part
// label" step works unmodified here. Deliberately kept free of any `'`/`,`
// octave marker on input — every note is a bare digit — so the "octave up
// shifted everything" assertion can require `'` on every shifted note
// without also matching a marker that was already there before the click.
const TWO_MEASURE_CLICK_TEST_SOURCE = [
  '# metadata',
  'title = "part label click test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0
  '[H] 5 6',
  '',
  '[M] 3 4', // measure 1
  '[H] 7 1',
].join('\n')

async function loadSource(page: import('@playwright/test').Page, src: string) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'test.jianpu',
        userFiles: { 'test.jianpu': source },
        bin: {},
        fileIds: { 'test.jianpu': crypto.randomUUID() },
      }),
    )
  }, src)
}

async function waitForEditor(page: import('@playwright/test').Page) {
  await page.waitForSelector('.monaco-editor .view-lines', { timeout: 30_000 })
}

/** Finds the byte offsets of `startNeedle`/`endNeedle` (searched from the
 * first `[Key]`-prefixed data line onward, so a short needle like "4" can't
 * spuriously match the directive line's `time=4/4` above it — both needles
 * must appear exactly once in whatever fixture calls this) and returns the
 * on-screen pixel center of each, via Monaco's own coordinate mapping. */
async function findDragCoords(
  page: import('@playwright/test').Page,
  startNeedle: string,
  endNeedle: string,
) {
  return page.evaluate(
    ({ startNeedle, endNeedle }) => {
      const ed = window.monaco?.editor.getEditors()[0]
      const model = ed?.getModel()
      if (!ed || !model) throw new Error('editor not mounted')
      const text = model.getValue()
      const dataLinesStart = text.indexOf('[')
      const startIndex = text.indexOf(startNeedle, dataLinesStart)
      const endIndex = text.indexOf(endNeedle, startIndex) + endNeedle.length
      const startPos = model.getPositionAt(startIndex)
      const endPos = model.getPositionAt(endIndex)
      const editorRect = ed.getDomNode()?.getBoundingClientRect()
      if (!editorRect) throw new Error('editor has no dom node')
      const startCoords = ed.getScrolledVisiblePosition(startPos)
      const endCoords = ed.getScrolledVisiblePosition(endPos)
      if (!startCoords || !endCoords) throw new Error('position not visible')
      return {
        start: {
          x: editorRect.left + startCoords.left,
          y: editorRect.top + startCoords.top + startCoords.height / 2,
        },
        end: {
          x: editorRect.left + endCoords.left,
          y: editorRect.top + endCoords.top + endCoords.height / 2,
        },
      }
    },
    { startNeedle, endNeedle },
  )
}

/** Selects the source text between two needles (see `findDragCoords`) with
 * a genuine mouse drag over the editor's DOM — not Monaco's `setSelection`
 * API — so this exercises the exact same mouse-driven path a person
 * dragging across a measure/part boundary would. */
async function selectRangeBetween(
  page: import('@playwright/test').Page,
  startNeedle: string,
  endNeedle: string,
) {
  // Wait for `noteSpans` to have loaded (the async `listNoteSpans` worker
  // round-trip, see `useJianpuWorkerRenderRequests.ts`) before dragging —
  // otherwise `useNoteSelection`'s `handleEditorSelectionChange` derives the
  // SVG highlight from an empty span list, and since it only re-derives on
  // the *next* selection-change event, a selection made before spans load
  // never gets a highlight even after they arrive.
  await page.waitForSelector(
    `rect[data-variant="${DATA_VARIANT.noteClickTarget}"]`,
    { timeout: 10_000 },
  )
  // The rects existing in the DOM only means the SVG has rendered — it says
  // nothing about whether `noteSpans` (the separate, debounced `listNoteSpans`
  // worker round-trip `handleEditorSelectionChange` reads to resolve a text
  // selection into highlighted notes) has arrived yet. There's no DOM signal
  // for that, so give it a beat, matching the wait
  // `note-drag-select-highlight.steps.ts` uses for the same round-trip.
  await page.waitForTimeout(700)
  const { start, end } = await findDragCoords(page, startNeedle, endNeedle)
  await page.mouse.move(start.x, start.y)
  await page.mouse.down()
  await page.mouse.move(end.x, end.y, { steps: 10 })
  await page.mouse.up()
}

Given('the single-measure melody-bass fixture is loaded', async ({ page }) => {
  await loadSource(page, SINGLE_MEASURE_SOURCE)
  await page.goto('/')
  await waitForEditor(page)
})

Given(
  'the single-measure tied-repeat-note fixture is loaded',
  async ({ page }) => {
    await loadSource(page, SINGLE_MEASURE_TIED_REPEAT_NOTE_SOURCE)
    await page.goto('/')
    await waitForEditor(page)
  },
)

Given('the two-measure melody fixture is loaded', async ({ page }) => {
  await loadSource(page, TWO_MEASURE_SOURCE)
  await page.goto('/')
  await waitForEditor(page)
})

Given(
  'the two-measure melody-harmony click-test fixture is loaded',
  async ({ page, focusEditor }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'part-label-click-test.jianpu',
          userFiles: { 'part-label-click-test.jianpu': source },
          bin: {},
          fileIds: {
            'part-label-click-test.jianpu': 'part-label-click-test-id-001',
          },
        }),
      )
    }, TWO_MEASURE_CLICK_TEST_SOURCE)
    await page.goto('/')
    await waitForEditor(page)
    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="part-label"][data-part-index="1"]', {
      timeout: 10_000,
    })
    // Primes measureSpans (same priming dance the part-label-click specs
    // use) so the SVG has settled before the click step hit-tests it.
    await focusEditor()
    await page.keyboard.press('Control+g')
    await page.keyboard.type('10')
    await page.keyboard.press('Enter')
    await expect(page.locator('button.play-measure-btn')).toHaveText(
      /Measure/,
      { timeout: 5_000 },
    )
    await expect(
      page.locator('.preview-page [data-testid="measure-highlight"]').first(),
    ).toBeVisible({ timeout: 5_000 })
  },
)

When('I select {string} on the Melody line', async ({ page }, text: string) => {
  await selectRangeBetween(page, text, text)
})

When(
  'I select from {string} to {string} across the blank line separating the measures',
  async ({ page }, startNeedle: string, endNeedle: string) => {
    await selectRangeBetween(page, startNeedle, endNeedle)
  },
)

When(
  'I select from {string} on the Melody line to {string} on the Bass line',
  async ({ page }, startNeedle: string, endNeedle: string) => {
    await selectRangeBetween(page, startNeedle, endNeedle)
  },
)

// Sets the selection directly via the Monaco API instead of a physical mouse
// drag (contrast `selectRangeBetween`). The "keeps its own shape" scenario
// only cares that the *shape* of an already-continuous selection survives an
// octave shift, not that a drag gesture produced it — and the pixel-
// coordinate drag this file's other steps use (`selectRangeBetween`) has a
// documented pre-existing flake (see `HANDOFF-selection-octave-toolbar.md`)
// that occasionally lands the selection a character or more off, which would
// make this invariant's pass/fail depend on drag-accuracy luck rather than
// the thing actually under test.
When(
  'I precisely select from {string} to {string} spanning the two measures',
  async ({ page }, startNeedle: string, endNeedle: string) => {
    await page.evaluate(
      ({ startNeedle, endNeedle }) => {
        const ed = window.monaco?.editor.getEditors()[0]
        const model = ed?.getModel()
        const monacoApi = window.monaco
        if (!ed || !model || !monacoApi) throw new Error('editor not mounted')
        const text = model.getValue()
        const dataLinesStart = text.indexOf('[')
        const startIndex = text.indexOf(startNeedle, dataLinesStart)
        const endIndex = text.indexOf(endNeedle, startIndex) + endNeedle.length
        const startPos = model.getPositionAt(startIndex)
        const endPos = model.getPositionAt(endIndex)
        ed.setSelection(
          new monacoApi.Selection(
            startPos.lineNumber,
            startPos.column,
            endPos.lineNumber,
            endPos.column,
          ),
        )
        ed.focus()
      },
      { startNeedle, endNeedle },
    )
  },
)

When(
  'I precisely select the disjoint notes {string} in the first measure and {string} in the second',
  async ({ page }, firstNeedle: string, secondNeedle: string) => {
    // Same `noteSpans`-readiness wait `selectRangeBetween` needs (see its own
    // doc comment) — this step drives the selection via the Monaco API
    // directly rather than a mouse drag, but still ends up going through the
    // same `handleEditorSelectionChange` path that depends on `noteSpans`
    // having loaded. Longer than that one because this step performs no
    // mouse movement at all to eat up incidental real time first, so under
    // heavy parallel test-worker load the round-trip can still be pending
    // right at the 700ms mark.
    await page.waitForTimeout(1500)
    await page.evaluate(
      ({ firstNeedle, secondNeedle }) => {
        const ed = window.monaco?.editor.getEditors()[0]
        const model = ed?.getModel()
        const monacoApi = window.monaco
        if (!ed || !model || !monacoApi) throw new Error('editor not mounted')
        const text = model.getValue()
        const dataLinesStart = text.indexOf('[')
        const firstIndex = text.indexOf(firstNeedle, dataLinesStart)
        const secondIndex = text.indexOf(
          secondNeedle,
          firstIndex + firstNeedle.length,
        )
        const toSelection = (index: number, needle: string) => {
          const startPos = model.getPositionAt(index)
          const endPos = model.getPositionAt(index + needle.length)
          return new monacoApi.Selection(
            startPos.lineNumber,
            startPos.column,
            endPos.lineNumber,
            endPos.column,
          )
        }
        ed.setSelections([
          toSelection(firstIndex, firstNeedle),
          toSelection(secondIndex, secondNeedle),
        ])
        ed.focus()
      },
      { firstNeedle, secondNeedle },
    )
  },
)

When(
  'I place the caret on the Melody line without selecting a range',
  async ({ page, focusEditor }) => {
    await focusEditor()
    await page.evaluate(() => {
      const ed = window.monaco?.editor.getEditors()[0]
      const model = ed?.getModel()
      if (!ed || !model) return
      const text = model.getValue()
      const pos = model.getPositionAt(text.indexOf('[Melody]'))
      ed.setPosition(pos)
    })
  },
)
