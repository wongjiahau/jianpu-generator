import { expect, type Locator } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then } from './fixtures'

/**
 * A measure's directive line (bar number + label/bpm/key/time-signature) is
 * currently positioned using a column width computed from the measure's
 * musical content alone (`block_column_width`), never the directive line's
 * own rendered text width. When the directive text is wider than the notes
 * (e.g. a single whole note under a `label=/bpm=/key=/time=` directive),
 * it overflows past the measure's bar line and collides with the next
 * measure's own directive line.
 *
 * Measure 0: a single note ("1", auto-extended to fill the 4/4 bar — see
 * "a shortfall extends the last note" in syntax.md) renders as one narrow
 * column, under a directive line carrying a label plus bpm/key/time — far
 * wider than that one column.
 * Measure 1: a full bar of 4 quarter notes (naturally wide), with its own
 * (short) directive line triggered by a key change, so both measures render
 * a directive line in the same system for the overlap check.
 */
const source = [
  '# metadata',
  'title = "directive width overflow test"',
  'max_measures_per_system = 2',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  'bpm=92 key=C4 time=4/4 label="Verse 1"',
  '[M] 1', // measure 0 — one note, auto-extended to fill the bar
  '',
  'key=D4',
  '[M] 2 3 4 5', // measure 1 — has its own directive line (key change)
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((source) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'directive-width-overflow-test.jianpu',
        userFiles: { 'directive-width-overflow-test.jianpu': source },
        bin: {},
        fileIds: {
          'directive-width-overflow-test.jianpu':
            'directive-width-overflow-test-id-001',
        },
      }),
    )
  }, source)
}

Given(
  'the directive-width-overflow test fixture is loaded',
  async ({ page }) => {
    await loadFixture(page)
    await page.goto('/')

    // Every emitted directive line renders its own bar number, so 2 measures
    // each with a directive line produce 4 `directive-line` text elements
    // (bar number + trailing spans, per measure) in document order.
    await expect(
      page.locator('text[data-variant="directive-line"]'),
    ).toHaveCount(4, { timeout: 10_000 })
  },
)

Then(
  "the first measure's directive line does not overlap the second measure's directive line",
  async ({ page }) => {
    const directiveTexts = page.locator('text[data-variant="directive-line"]')

    function requireBox(box: Awaited<ReturnType<Locator['boundingBox']>>) {
      if (!box) {
        throw new Error(
          'Could not get bounding box for a directive line text element.',
        )
      }
      return box
    }

    const firstMeasureBoxes = (
      await Promise.all(
        [0, 1].map((i) => stableBoundingBox(directiveTexts.nth(i))),
      )
    ).map(requireBox)
    const secondMeasureBoxes = (
      await Promise.all(
        [2, 3].map((i) => stableBoundingBox(directiveTexts.nth(i))),
      )
    ).map(requireBox)

    const firstMeasureRightEdge = Math.max(
      ...firstMeasureBoxes.map((box) => box.x + box.width),
    )
    const secondMeasureLeftEdge = Math.min(
      ...secondMeasureBoxes.map((box) => box.x),
    )

    expect(firstMeasureRightEdge).toBeLessThanOrEqual(secondMeasureLeftEdge)
  },
)

/**
 * Regression: even after the single-directive fix above, a measure's
 * directive line still collided with the NEXT measure's directive line when
 * that next measure ALSO carried its own directive line — because a
 * measure's directive line is anchored at (and starts drawing from) the
 * *previous* measure's own trailing bar line (see
 * `layout_decoration::make_decoration_row`'s doc comment), a column region
 * the previous fix folded entirely into that previous measure's own rod.
 *
 * Three measures, each under its own `label=` directive (mirroring the
 * user's hand-confirmed repro): measure 0 additionally carries
 * bpm/key/time, measures 1 and 2 each trigger their own directive line via
 * a fresh `label=`.
 */
const twoAdjacentDirectivesSource = [
  '# metadata',
  'title = "two adjacent directives test"',
  'max_measures_per_system = 3',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  'bpm=92 key=C4 time=4/4 label="Verse 1 begins here"',
  '[M] 1',
  '',
  'label="Pre-Chorus transition"',
  '[M] 2',
  '',
  'label="Chorus hits hard"',
  '[M] 3',
].join('\n')

Given(
  'the two-adjacent-directives test fixture is loaded',
  async ({ page }) => {
    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'two-adjacent-directives-test.jianpu',
          userFiles: { 'two-adjacent-directives-test.jianpu': source },
          bin: {},
          fileIds: {
            'two-adjacent-directives-test.jianpu':
              'two-adjacent-directives-test-id-001',
          },
        }),
      )
    }, twoAdjacentDirectivesSource)
    await page.goto('/')

    // Every measure here carries a `label=` directive, so 3 measures produce
    // 3 section-label groups (one click-target rect each).
    await expect(
      page.locator('rect[data-variant="section-label-click-target-rect"]'),
    ).toHaveCount(3, { timeout: 10_000 })
  },
)

Then(
  "no measure's directive line overlaps the next measure's directive line",
  async ({ page }) => {
    const sectionLabelGroups = page.locator('g[data-tag="section-label"]')
    const groupCount = await sectionLabelGroups.count()
    expect(groupCount).toBe(3)

    // Each measure's directive line (bar number, section-label click
    // target, section-label text, trailing key/bpm/time spans) is wrapped
    // in one `g[data-tag="section-label"]` group — see
    // `render_directive_line`/`render_section_label_group`. Checking each
    // group's own directive-line text spans AND its click-target rect
    // (rather than just the group's overall bounding box) matches what
    // actually collided: the click-target rect alone already spans the
    // full directive line, but the previous fix's regression test only
    // ever looked at `text[data-variant="directive-line"]`, which excludes
    // the section-label text itself.
    // A directive line's trailing key/bpm/time spans always render as one
    // `<text>` element even when none of those fields are set (empty
    // `spans`) — Chrome reports a degenerate, meaningless bounding box (zero
    // width/height, positioned nowhere near the actual line) for a `<text>`
    // with no glyphs, so those must be filtered out before taking a
    // min/max across a group's boxes.
    async function groupBoxes(groupIndex: number) {
      const group = sectionLabelGroups.nth(groupIndex)
      const boxes = await Promise.all(
        [
          ...(await group.locator('text[data-variant="directive-line"]').all()),
          ...(await group
            .locator('rect[data-variant="section-label-click-target-rect"]')
            .all()),
        ].map((locator) => stableBoundingBox(locator)),
      )
      return boxes.filter(
        (box): box is NonNullable<typeof box> =>
          box !== null && box.width > 0 && box.height > 0,
      )
    }

    async function rightEdge(groupIndex: number): Promise<number> {
      const boxes = await groupBoxes(groupIndex)
      expect(boxes.length).toBeGreaterThan(0)
      return Math.max(...boxes.map((box) => box.x + box.width))
    }

    async function leftEdge(groupIndex: number): Promise<number> {
      const boxes = await groupBoxes(groupIndex)
      expect(boxes.length).toBeGreaterThan(0)
      return Math.min(...boxes.map((box) => box.x))
    }

    for (let i = 0; i < groupCount - 1; i++) {
      const currentRightEdge = await rightEdge(i)
      const nextLeftEdge = await leftEdge(i + 1)
      expect(currentRightEdge).toBeLessThanOrEqual(nextLeftEdge)
    }
  },
)
