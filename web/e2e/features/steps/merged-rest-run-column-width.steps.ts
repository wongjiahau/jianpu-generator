import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

type MeasureSpec = { kind: 'dense'; noteCount: number } | { kind: 'rest' }

let state: {
  maxMeasuresPerSystem: number
  measures: MeasureSpec[]
} = { maxMeasuresPerSystem: 4, measures: [] }

function ensureMeasure(index: number): MeasureSpec {
  while (state.measures.length <= index) {
    state.measures.push({ kind: 'rest' })
  }
  return state.measures[index]
}

/** `n` sixteenth notes (`=` suffix), cycling pitches 1-7, summing to exactly
 * `n` quarter-beats — a full measure at `n = 16` (4/4's capacity, see
 * "Duration" in syntax.md). */
function denseMeasureTokens(noteCount: number): string {
  return Array.from({ length: noteCount }, (_, i) => `${(i % 7) + 1}=`).join(
    ' ',
  )
}

function buildSource(): string {
  const lines: string[] = [
    '# metadata',
    'title = "merged rest run column width test"',
    `max_measures_per_system = ${state.maxMeasuresPerSystem}`,
    '',
    '# parts',
    'Melody [M] = notes',
    '',
    '# score',
  ]
  for (const measure of state.measures) {
    const tokens =
      measure.kind === 'dense'
        ? denseMeasureTokens(measure.noteCount)
        : '0 0 0 0'
    lines.push(`[M] ${tokens}`, '')
  }
  return lines.join('\n')
}

Given('parts Melody [M] are declared', async () => {
  state = { maxMeasuresPerSystem: 4, measures: [] }
})

Given("the score's max_measures_per_system is {int}", async ({}, n: number) => {
  state.maxMeasuresPerSystem = n
})

Given(
  'measure {int} is a dense measure of {int} sixteenth notes',
  async ({}, index: number, noteCount: number) => {
    ensureMeasure(index)
    state.measures[index] = { kind: 'dense', noteCount }
  },
)

Given(
  'measures {int} to {int} are all-rest, merging into one run',
  async ({}, from: number, to: number) => {
    for (let i = from; i <= to; i++) {
      ensureMeasure(i)
      state.measures[i] = { kind: 'rest' }
    }
  },
)

When('the merged-rest-run-column-width score is laid out', async ({ page }) => {
  const source = buildSource()
  await page.addInitScript((src) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'merged-rest-run-column-width-test.jianpu',
        userFiles: { 'merged-rest-run-column-width-test.jianpu': src },
        bin: {},
        fileIds: {
          'merged-rest-run-column-width-test.jianpu':
            'merged-rest-run-column-width-test-id-001',
        },
      }),
    )
  }, source)

  await page.goto('/')
  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
  await page.waitForSelector('[data-tag="measure"]', { timeout: 10_000 })
})

/** The count label is the only text node in this fixture with this exact
 * content (every other visible note glyph is a single digit 1-7; the
 * count is always multi-digit here). `:text-is()` matches the element's
 * full text content exactly, so it won't also match a substring inside a
 * longer label. */
function mergedRestCountLabel(
  page: import('@playwright/test').Page,
  count: string,
) {
  return page.locator(`svg text:text-is("${count}")`)
}

/** No content element carries a type marker in the live preview DOM (unlike
 * the static SVG exporter, `serializer/mod.rs`'s `data-variant`, which the
 * React renderer at `PreviewSvgRenderer.tsx` doesn't reproduce) — so the
 * merged rest's bar is identified by shape instead: `render_multi_measure_rest`
 * draws its bar as a *horizontal* `<line>` (`y1 === y2`, nonzero length) at
 * stroke-width `row_height * 0.18` (~5.4 at the default 30pt row height) —
 * thick enough to stand apart from every other horizontal line the renderer
 * draws (a sixteenth-note beam/underline at stroke-width `1.0`, a tuplet
 * bracket's horizontal run at `0.5`; see `render_underline`/
 * `render_horizontal_line` in `glyph_renderers.rs`). */
async function mergedRestBarBoundingBox(page: import('@playwright/test').Page) {
  const box = await page.evaluate(() => {
    const lines = Array.from(document.querySelectorAll('svg line'))
    const horizontal = lines.filter((l) => {
      const strokeWidth = Number(l.getAttribute('stroke-width'))
      return (
        l.getAttribute('y1') === l.getAttribute('y2') &&
        l.getAttribute('x1') !== l.getAttribute('x2') &&
        strokeWidth > 2
      )
    })
    if (horizontal.length !== 1) {
      throw new Error(
        `Expected exactly one thick horizontal line (the merged rest bar), found ${horizontal.length}.`,
      )
    }
    const rect = horizontal[0].getBoundingClientRect()
    return { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
  })
  return box
}

function mergedRestMeasureIndexRange(): { start: number; end: number } {
  const restIndices = state.measures
    .map((m, i) => ({ m, i }))
    .filter(({ m }) => m.kind === 'rest')
    .map(({ i }) => i)
  return {
    start: Math.min(...restIndices),
    end: Math.max(...restIndices),
  }
}

function mergedRestRunCount(): number {
  const { start, end } = mergedRestMeasureIndexRange()
  return end - start + 1
}

Then(
  'the merged rest run shows the count {string}',
  async ({ page }, expected: string) => {
    await expect(mergedRestCountLabel(page, expected)).toHaveCount(1)
  },
)

Then(
  "the count label's rendered width is no wider than the merged rest bar's rendered width",
  async ({ page }) => {
    const barBox = await mergedRestBarBoundingBox(page)
    const labelBox = await stableBoundingBox(
      mergedRestCountLabel(page, String(mergedRestRunCount())),
    )
    if (!labelBox)
      throw new Error('Could not get bounding box for the count label.')
    expect(labelBox.width).toBeLessThanOrEqual(barBox.width)
  },
)

async function mergedRestMeasureBox(page: import('@playwright/test').Page) {
  const { start, end } = mergedRestMeasureIndexRange()
  const measureBox = await stableBoundingBox(
    page
      .locator(
        `[data-tag="measure"][data-measure-index="${start}"][data-measure-index-end="${end}"]`,
      )
      .first(),
  )
  if (!measureBox) {
    throw new Error('Could not get bounding box for the merged rest measure.')
  }
  return measureBox
}

Then(
  "the count label stays within the merged rest measure's own click-target bounds",
  async ({ page }) => {
    const measureBox = await mergedRestMeasureBox(page)
    const labelBox = await stableBoundingBox(
      mergedRestCountLabel(page, String(mergedRestRunCount())),
    )
    if (!labelBox)
      throw new Error('Could not get bounding box for the count label.')

    expect(labelBox.x).toBeGreaterThanOrEqual(measureBox.x)
    expect(labelBox.x + labelBox.width).toBeLessThanOrEqual(
      measureBox.x + measureBox.width,
    )
  },
)

Then(
  'the merged rest bar keeps horizontal padding from the measure dividers on both sides',
  async ({ page }) => {
    // `resolve_multi_measure_rest` insets the drawn bar by `GLYPH_LEFT_PADDING`
    // from both ends of its reserved column span — this checks the bar's own
    // ink (not just its reserved column region) actually stays clear of the
    // click-target region's edges, i.e. of the adjacent measure dividers.
    const measureBox = await mergedRestMeasureBox(page)
    const barBox = await mergedRestBarBoundingBox(page)
    const minPaddingPx = 0.5

    expect(barBox.x).toBeGreaterThan(measureBox.x + minPaddingPx)
    expect(barBox.x + barBox.width).toBeLessThan(
      measureBox.x + measureBox.width - minPaddingPx,
    )
  },
)
