import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/** Harmony is declared second, so it's `source_part_index` 1. */
const HARMONY_PART_INDEX = 1

let state: {
  hideRestingParts: boolean
  melodyNotes: string
  /** `null` = no `[H]` line at all (measure 0 has no Harmony line). */
  harmonyNotes: string | null
} = { hideRestingParts: false, melodyNotes: '1 2 3 4', harmonyNotes: null }

Given('parts Melody [M], Harmony [H] are declared', async () => {
  state = {
    hideRestingParts: false,
    melodyNotes: '1 2 3 4',
    harmonyNotes: null,
  }
})

Given('hide_resting_parts is {string}', async ({}, value: string) => {
  state.hideRestingParts = value === 'yes'
})

Given(
  "measure {int}'s Melody line has notes {string}",
  async ({}, _index: number, notes: string) => {
    state.melodyNotes = notes
  },
)

Given('measure {int} has no Harmony line', async ({}, _index: number) => {
  state.harmonyNotes = null
})

Given(
  "measure {int}'s Harmony line has notes {string}",
  async ({}, _index: number, notes: string) => {
    state.harmonyNotes = notes
  },
)

function buildSource(): string {
  const lines: string[] = [
    '# metadata',
    'title = "omitted part rest glyph test"',
    `hide_resting_parts = ${state.hideRestingParts ? 'yes' : 'no'}`,
    '',
    '# parts',
    'Melody [M] = notes',
    'Harmony [H] = notes',
    '',
    '# score',
    `[M] ${state.melodyNotes}`,
  ]
  if (state.harmonyNotes !== null) {
    lines.push(`[H] ${state.harmonyNotes}`)
  }
  return lines.join('\n')
}

When('the omitted-part-rest-glyph score is laid out', async ({ page }) => {
  const source = buildSource()
  await page.addInitScript((src) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'omitted-part-rest-glyph-test.jianpu',
        userFiles: { 'omitted-part-rest-glyph-test.jianpu': src },
        bin: {},
        fileIds: {
          'omitted-part-rest-glyph-test.jianpu':
            'omitted-part-rest-glyph-test-id-001',
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

/** The rectangular region measure `index`'s Harmony row occupies: x-bounds
 * from the measure's own click-target rect, y-bounds from the Harmony
 * part-label's rendered box (one row tall, vertically centered on the row
 * like every note/rest glyph in it). */
async function harmonyRowRegion(
  page: import('@playwright/test').Page,
  index: number,
) {
  const measureBox = await stableBoundingBox(
    page
      .locator(
        `[data-tag="measure"][data-measure-index="${index}"][data-measure-index-end="${index}"]`,
      )
      .first(),
  )
  if (!measureBox) throw new Error(`Could not find measure ${index}.`)

  const labelBox = await stableBoundingBox(
    page
      .locator(
        `[data-tag="part-label"][data-part-index="${HARMONY_PART_INDEX}"]`,
      )
      .first(),
  )
  if (!labelBox) throw new Error('Could not find the Harmony part label.')

  return {
    x: measureBox.x,
    width: measureBox.width,
    y: labelBox.y,
    height: labelBox.height,
  }
}

/** Whether the inverted-hat placeholder glyph (see `render_omitted_part_rest`
 * in `glyph_renderers_rest.rs`) is present within `region`: a *pair* of
 * short horizontal `<line>` elements, stacked close together, one much
 * thicker than the other (the cap and the solid block beneath it) — drawn
 * with no `data-variant` in this live-preview DOM (unlike the static SVG
 * exporter's `data-variant="omitted-part-rest"`; the React renderer's
 * `<line>` case in `PreviewSvgRenderer.tsx` doesn't set one), so shape is
 * the only signal available here. */
async function hasOmittedPartRestGlyph(
  page: import('@playwright/test').Page,
  region: { x: number; y: number; width: number; height: number },
): Promise<boolean> {
  return page.evaluate((r) => {
    const lines = Array.from(document.querySelectorAll('svg line'))
    const candidates = lines.filter((l) => {
      const x1 = Number(l.getAttribute('x1'))
      const y1 = Number(l.getAttribute('y1'))
      const y2 = Number(l.getAttribute('y2'))
      const x2 = Number(l.getAttribute('x2'))
      const isHorizontal = y1 === y2 && x1 !== x2
      if (!isHorizontal) return false
      const rect = l.getBoundingClientRect()
      const cx = rect.x + rect.width / 2
      const cy = rect.y + rect.height / 2
      return (
        cx >= r.x && cx <= r.x + r.width && cy >= r.y && cy <= r.y + r.height
      )
    })
    if (candidates.length < 2) return false
    const strokeWidths = candidates
      .map((l) => Number(l.getAttribute('stroke-width')))
      .sort((a, b) => a - b)
    // The block's stroke is drawn much thicker than the cap's (see
    // `render_omitted_part_rest`: `block_stroke_width` is roughly
    // `cap_stroke_width * 3.6`) — a written `0`/other glyphs in this same
    // region don't produce this two-tier, closely-stacked pairing.
    const thinnest = strokeWidths[0]
    const thickest = strokeWidths[strokeWidths.length - 1]
    return thinnest > 0 && thickest > thinnest * 2
  }, region)
}

async function hasZeroRestText(
  page: import('@playwright/test').Page,
  region: { x: number; y: number; width: number; height: number },
): Promise<boolean> {
  return page.evaluate((r) => {
    const texts = Array.from(document.querySelectorAll('svg text'))
    return texts.some((t) => {
      if (!(t.textContent ?? '').startsWith('0')) return false
      const rect = t.getBoundingClientRect()
      const cx = rect.x + rect.width / 2
      const cy = rect.y + rect.height / 2
      return (
        cx >= r.x && cx <= r.x + r.width && cy >= r.y && cy <= r.y + r.height
      )
    })
  }, region)
}

Then(
  'the Harmony row in measure {int} shows the inverted-hat placeholder glyph',
  async ({ page }, index: number) => {
    const region = await harmonyRowRegion(page, index)
    expect(await hasOmittedPartRestGlyph(page, region)).toBe(true)
  },
)

Then(
  'the Harmony row in measure {int} does not show the inverted-hat placeholder glyph',
  async ({ page }, index: number) => {
    const region = await harmonyRowRegion(page, index)
    expect(await hasOmittedPartRestGlyph(page, region)).toBe(false)
  },
)

Then(
  'the Harmony row in measure {int} shows a {string} rest glyph',
  async ({ page }, index: number, _expected: string) => {
    const region = await harmonyRowRegion(page, index)
    expect(await hasZeroRestText(page, region)).toBe(true)
  },
)

Then(
  'the Harmony row in measure {int} does not show a {string} rest glyph',
  async ({ page }, index: number, _expected: string) => {
    const region = await harmonyRowRegion(page, index)
    expect(await hasZeroRestText(page, region)).toBe(false)
  },
)
