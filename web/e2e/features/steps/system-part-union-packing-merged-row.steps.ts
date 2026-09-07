import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Then, When } from './fixtures'
import { partLabelsFor } from './system-part-union-packing.fixture'

When("I Ctrl-click {word}'s part label", async ({ page }, part: string) => {
  const label = partLabelsFor(page, part).first()
  const box = await stableBoundingBox(label)
  if (!box) throw new Error(`Could not get bounding box for ${part}'s label.`)
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.up()
  await page.keyboard.up('Control')
})

Then('all notes in the first system are highlighted', async ({ page }) => {
  // "The first system" is every part row whose label starts at measure 0,
  // spanning measures 0..=that label's own `data-measure-index-end` (every
  // label in one system shares the same start/end — see the system-scoping
  // comment on `PartLabelHit` in `previewLabelRangeHighlights.ts`). Deliberately
  // generic rather than naming a specific part/measure: the bug this guards
  // against is that a part whose row got absorbed into another part's row
  // (see `consolidate_rows`) renders its measure as bare, non-interactive
  // duplicate glyphs (`make_padding_row`) with no click-target `rect` at
  // all, so nothing at that glyph's position can ever carry
  // `data-note-range-selected` — asserting over *every* glyph in the system
  // catches that regardless of which part/measure it lands on.
  const firstSystemLabels = page.locator(
    '[data-tag="part-label"][data-measure-index-start="0"]',
  )
  const labelCount = await firstSystemLabels.count()
  if (labelCount === 0) {
    throw new Error('No part labels found for the first system.')
  }
  const rowBoxes: { y: number; height: number }[] = []
  let systemEnd = 0
  for (let i = 0; i < labelCount; i++) {
    const label = firstSystemLabels.nth(i)
    const box = await stableBoundingBox(label)
    if (!box) continue
    rowBoxes.push(box)
    const end = await label.getAttribute('data-measure-index-end')
    systemEnd = Math.max(systemEnd, Number.parseInt(end ?? '0', 10))
  }

  const measureBoxes: { x: number; width: number }[] = []
  for (let m = 0; m <= systemEnd; m++) {
    const box = await stableBoundingBox(
      page.locator(`[data-tag="measure"][data-measure-index="${m}"]`),
    )
    if (box) measureBoxes.push(box)
  }

  const inFirstSystem = (cx: number, cy: number) =>
    measureBoxes.some((m) => cx >= m.x && cx <= m.x + m.width) &&
    rowBoxes.some((r) => cy >= r.y && cy <= r.y + r.height)

  const highlightedRectBoxes = await page.evaluate(() =>
    Array.from(
      document.querySelectorAll<SVGRectElement>(
        '[data-tag="note"][data-note-range-selected] rect[data-variant="note-click-target-rect"]',
      ),
    ).map((el) => el.getBoundingClientRect().toJSON()),
  )

  const digitGlyphs = page.locator('text').getByText(/^[1-7]$/)
  const glyphCount = await digitGlyphs.count()
  let checked = 0
  for (let i = 0; i < glyphCount; i++) {
    const box = await stableBoundingBox(digitGlyphs.nth(i))
    if (!box) continue
    const centerX = box.x + box.width / 2
    const centerY = box.y + box.height / 2
    if (!inFirstSystem(centerX, centerY)) continue
    checked += 1
    const isHighlighted = highlightedRectBoxes.some(
      (r) =>
        centerX >= r.x &&
        centerX <= r.x + r.width &&
        centerY >= r.y &&
        centerY <= r.y + r.height,
    )
    expect(isHighlighted).toBe(true)
  }
  expect(checked).toBeGreaterThan(0)
})

Then(
  "measure {int} has a note glyph in {word}'s row",
  async ({ page }, index: number, part: string) => {
    // Mirror image of "no rest glyph": a note digit glyph renders as a bare
    // `<text>` with content `1`-`7` (see `pitch.to_digit()` in
    // `src/ast/parsed/pitch.rs`), regardless of whether the element carries
    // a `note_id` (a decorative/padded glyph with `note_id: None` still
    // renders the digit — see `render_note_head` in
    // `src/renderer/new_renderer/glyph_renderers.rs` — it just has no
    // wrapping click-target group). So "is this part's row genuinely
    // showing its notes here, not left blank" has to be answered the same
    // geometric way as the rest-glyph check: does any digit-1-7 glyph's
    // center fall inside the intersection of this measure's column and this
    // part's row?
    const measureBox = await stableBoundingBox(
      page.locator(`[data-tag="measure"][data-measure-index="${index}"]`),
    )
    const rowBox = await stableBoundingBox(partLabelsFor(page, part).first())
    if (!measureBox || !rowBox) {
      throw new Error(
        `Could not get bounding boxes for measure ${index} / ${part}'s row.`,
      )
    }
    const notes = page.locator('text').getByText(/^[1-7]$/)
    const count = await notes.count()
    let found = false
    for (let i = 0; i < count; i++) {
      const box = await stableBoundingBox(notes.nth(i))
      if (!box) continue
      const centerX = box.x + box.width / 2
      const centerY = box.y + box.height / 2
      const inMeasure =
        centerX >= measureBox.x && centerX <= measureBox.x + measureBox.width
      const inRow = centerY >= rowBox.y && centerY <= rowBox.y + rowBox.height
      if (inMeasure && inRow) {
        found = true
        break
      }
    }
    expect(found).toBe(true)
  },
)

Then(
  "measure {int} has no rest glyph in {word}'s row",
  async ({ page }, index: number, part: string) => {
    // A rest glyph renders as a bare `<text>0</text>` with no wrapping
    // `[data-tag]` group and, in the live preview (unlike the static SVG
    // export from `src/serializer/mod.rs`), no `data-variant` attribute
    // either — `PreviewSvgRenderer.tsx` only stamps `data-variant` on
    // click-target `rect`s (see `transparentRectRoleToDataVariant`), not on
    // glyph text. A padded row's elements carry `note_id: None` (see
    // `make_padding_row` in `src/grid_layout/layout_systems.rs`), so there's
    // no click-target/playback-cursor group to key off either. So "is there
    // a rest in this measure/row" has to be answered by matching literal
    // text content `"0"` (jianpu never uses digit 0 for an actual note, only
    // for a rest) plus geometry: does any such glyph's center fall inside
    // the intersection of this measure's column (x-range, from the
    // `[data-tag="measure"]` click-target rect) and this part's row
    // (y-range, from its part-label click-target rect)?
    const measureBox = await stableBoundingBox(
      page.locator(`[data-tag="measure"][data-measure-index="${index}"]`),
    )
    const rowBox = await stableBoundingBox(partLabelsFor(page, part).first())
    if (!measureBox || !rowBox) {
      throw new Error(
        `Could not get bounding boxes for measure ${index} / ${part}'s row.`,
      )
    }
    const rests = page.locator('text').getByText('0', { exact: true })
    const count = await rests.count()
    for (let i = 0; i < count; i++) {
      const box = await stableBoundingBox(rests.nth(i))
      if (!box) continue
      const centerX = box.x + box.width / 2
      const centerY = box.y + box.height / 2
      const inMeasure =
        centerX >= measureBox.x && centerX <= measureBox.x + measureBox.width
      const inRow = centerY >= rowBox.y && centerY <= rowBox.y + rowBox.height
      expect(inMeasure && inRow).toBe(false)
    }
  },
)
