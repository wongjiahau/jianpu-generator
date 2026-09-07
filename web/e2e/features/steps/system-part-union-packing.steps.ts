import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'
import {
  ensureMeasure,
  loadFixture,
  NOTE_TOKEN_COUNT,
  PART_INDEX,
  partLabelAt,
  partLabelsFor,
  primeMeasureSpans,
  resetState,
  state,
} from './system-part-union-packing.fixture'

Given(
  'parts Melody [M], Harmony [H], Bass [B] are declared in that order',
  async () => {
    resetState()
  },
)

Given('max_measures_per_system is {int}', async ({}, n: number) => {
  state.maxMeasuresPerSystem = n
})

Given('hide_resting_parts is enabled', async () => {
  state.hideRestingParts = true
})

Given(
  'measures {int}-{int} each have notes for Melody and Harmony',
  async ({}, from: number, to: number) => {
    for (let i = from; i <= to; i++) {
      ensureMeasure(i).notesFor = ['Melody', 'Harmony']
    }
  },
)

Given('measure {int} has notes only for Melody', async ({}, index: number) => {
  ensureMeasure(index).notesFor = ['Melody']
})

Given('measure {int} has notes only for Harmony', async ({}, index: number) => {
  ensureMeasure(index).notesFor = ['Harmony']
})

Given('measure {int} has notes only for Bass', async ({}, index: number) => {
  ensureMeasure(index).notesFor = ['Bass']
})

Given(
  'measure {int} has notes for Melody and Harmony',
  async ({}, index: number) => {
    ensureMeasure(index).notesFor = ['Melody', 'Harmony']
  },
)

Given(
  'measure {int} has notes for Melody and Bass',
  async ({}, index: number) => {
    ensureMeasure(index).notesFor = ['Melody', 'Bass']
  },
)

Given('measures 0, 2, and 3 have notes for Melody and Harmony', async () => {
  for (const i of [0, 2, 3]) {
    ensureMeasure(i).notesFor = ['Melody', 'Harmony']
  }
})

Given(
  'measure {int} has notes only for Melody, with Harmony resting',
  async ({}, index: number) => {
    ensureMeasure(index).notesFor = ['Melody']
  },
)

Given(
  "measure {int}'s Melody part has {int} lyric verse",
  async ({}, index: number, verseCount: number) => {
    const measure = ensureMeasure(index)
    measure.notesFor = ['Melody']
    measure.melodyVerses = Array.from({ length: verseCount }, (_, v) => [
      `v${v}s1-${index}`,
      `v${v}s2-${index}`,
    ])
  },
)

Given(
  "measure {int}'s Melody part has {int} lyric verses",
  async ({}, index: number, verseCount: number) => {
    const measure = ensureMeasure(index)
    measure.notesFor = ['Melody']
    measure.melodyVerses = Array.from({ length: verseCount }, (_, v) => [
      `v${v}s1-${index}`,
      `v${v}s2-${index}`,
    ])
  },
)

Given(
  'measure {int} has identical Melody and Harmony notes with merge_duplicate_measures_across_parts enabled',
  async ({}, index: number) => {
    const measure = ensureMeasure(index)
    measure.notesFor = ['Melody', 'Harmony']
    measure.sameNotesAs = 'Melody'
    measure.directive = 'merge_duplicate_measures_across_parts=yes'
  },
)

Given(
  'measure {int} has identical Melody and Harmony notes with merge_duplicate_measures_across_parts disabled',
  async ({}, index: number) => {
    const measure = ensureMeasure(index)
    measure.notesFor = ['Melody', 'Harmony']
    measure.sameNotesAs = 'Melody'
    measure.directive = 'merge_duplicate_measures_across_parts=no'
  },
)

Given(
  'measure {int} has different notes for Melody and Harmony',
  async ({}, index: number) => {
    // Distinct from `measure {int} has notes for Melody and Harmony`: this
    // scenario's point is that Melody and Harmony are *not* mergeable here
    // (unlike an earlier identical measure in the same system), so it's
    // spelled out even though it currently sets the same default,
    // non-matching NOTE_TOKENS as that step.
    ensureMeasure(index).notesFor = ['Melody', 'Harmony']
  },
)

When('the score is laid out', async ({ page }) => {
  const firstMeasureLine = await loadFixture(page)
  await page.goto('/')

  await page.waitForSelector('[data-testid="play-measure-button"]', {
    timeout: 15_000,
  })
  await page.waitForSelector('[data-tag="part-label"][data-part-index="0"]', {
    timeout: 10_000,
  })
  await primeMeasureSpans(page, firstMeasureLine)
})

Then(
  "{word}'s part label spans measures {int} to {int} in one system",
  async ({ page }, part: string, from: number, to: number) => {
    const label = partLabelAt(page, part, from)
    await expect(label).toHaveCount(1, { timeout: 10_000 })
    await expect(label).toHaveAttribute('data-measure-index-end', String(to))
  },
)

Then(
  "{word}'s part label spans measures {int} to {int} in the same system",
  async ({ page }, part: string, from: number, to: number) => {
    const label = partLabelAt(page, part, from)
    await expect(label).toHaveCount(1, { timeout: 10_000 })
    await expect(label).toHaveAttribute('data-measure-index-end', String(to))
  },
)

Then(
  'measure {int} has no clickable {word} note',
  async ({ page }, _index: number, part: string) => {
    // A padded/synthetic full-measure rest carries `note_id: None` and
    // therefore emits no `[data-tag="note"]` click-target group at all (see
    // `src/grid_layout/click_targets.rs` / `src/serializer/mod.rs`). So the
    // absence of an extra group is verified by asserting the *total* count
    // of clickable notes for this part equals exactly the number of real
    // (non-padded) note tokens this fixture wrote for it — no more. Every
    // real note renders *two* `[data-tag="note"]` groups (one wrapping the
    // playback-cursor rect, one wrapping the click-target rect — see
    // `render_playback_cursor_target`/`render_note_click_target` in
    // `src/renderer/new_renderer/click_targets.rs`), hence the `* 2`.
    const totalReal = state.measures.filter((m) =>
      m.notesFor.includes(part),
    ).length
    await expect(
      page.locator(`[data-tag="note"][data-part-index="${PART_INDEX[part]}"]`),
    ).toHaveCount(totalReal * NOTE_TOKEN_COUNT * 2, { timeout: 10_000 })
  },
)

Then(
  "{word}'s part label appears twice, spanning measures {int}-{int} and {int}-{int}",
  async (
    { page },
    part: string,
    aStart: number,
    aEnd: number,
    bStart: number,
    bEnd: number,
  ) => {
    await expect(partLabelsFor(page, part)).toHaveCount(2, { timeout: 10_000 })
    const first = partLabelAt(page, part, aStart)
    await expect(first).toHaveCount(1)
    await expect(first).toHaveAttribute('data-measure-index-end', String(aEnd))
    const second = partLabelAt(page, part, bStart)
    await expect(second).toHaveCount(1)
    await expect(second).toHaveAttribute('data-measure-index-end', String(bEnd))
  },
)

Then(
  "Melody's, Harmony's, and Bass's part labels each span measures {int} to {int} in one system",
  async ({ page }, from: number, to: number) => {
    for (const part of ['Melody', 'Harmony', 'Bass']) {
      const label = partLabelAt(page, part, from)
      await expect(label).toHaveCount(1, { timeout: 10_000 })
      await expect(label).toHaveAttribute('data-measure-index-end', String(to))
    }
  },
)

Then(
  'the rows are ordered top to bottom: {word}, {word}',
  async ({ page }, first: string, second: string) => {
    const firstLabel = partLabelsFor(page, first).first()
    const secondLabel = partLabelsFor(page, second).first()
    await expect(firstLabel).toBeVisible({ timeout: 10_000 })
    await expect(secondLabel).toBeVisible({ timeout: 10_000 })
    const firstBox = await stableBoundingBox(firstLabel)
    const secondBox = await stableBoundingBox(secondLabel)
    if (!firstBox || !secondBox) {
      throw new Error(
        `Could not get bounding boxes for ${first}/${second} labels.`,
      )
    }
    expect(firstBox.y).toBeLessThan(secondBox.y)
  },
)

Then(
  'the rows are ordered top to bottom: {word}, {word}, {word}',
  async ({ page }, first: string, second: string, third: string) => {
    const boxes: number[] = []
    for (const part of [first, second, third]) {
      const label = partLabelsFor(page, part).first()
      await expect(label).toBeVisible({ timeout: 10_000 })
      const box = await stableBoundingBox(label)
      if (!box) {
        throw new Error(`Could not get bounding box for ${part} label.`)
      }
      boxes.push(box.y)
    }
    expect(boxes[0]).toBeLessThan(boxes[1])
    expect(boxes[1]).toBeLessThan(boxes[2])
  },
)

Then(
  "Bass's part label spans only measure {int} in a second system",
  async ({ page }, index: number) => {
    await expect(partLabelsFor(page, 'Bass')).toHaveCount(1, {
      timeout: 10_000,
    })
    const label = partLabelAt(page, 'Bass', index)
    await expect(label).toHaveCount(1, { timeout: 10_000 })
    await expect(label).toHaveAttribute('data-measure-index-end', String(index))
  },
)

Then('the first system has no Bass row', async ({ page }) => {
  await expect(
    page.locator(
      '[data-tag="part-label"][data-part-index="2"][data-measure-index-start="0"]',
    ),
  ).toHaveCount(0, { timeout: 10_000 })
})

Then(
  "Melody's verse-{int} lyric label spans measures {int} to {int} in one system",
  async ({ page }, verse: number, from: number, to: number) => {
    const label = page.locator(
      `[data-tag="lyric-label"][data-part-index="0"][data-verse="${verse}"][data-measure-index-start="${from}"]`,
    )
    await expect(label).toHaveCount(1, { timeout: 10_000 })
    await expect(label).toHaveAttribute('data-measure-index-end', String(to))
  },
)

Then(
  "Melody's verse-{int} lyric label spans measures {int} to {int} in the same system",
  async ({ page }, verse: number, from: number, to: number) => {
    const label = page.locator(
      `[data-tag="lyric-label"][data-part-index="0"][data-verse="${verse}"][data-measure-index-start="${from}"]`,
    )
    await expect(label).toHaveCount(1, { timeout: 10_000 })
    await expect(label).toHaveAttribute('data-measure-index-end', String(to))
  },
)

Then(
  'measure {int} has no clickable verse-{int} lyric',
  async ({ page }, _index: number, verse: number) => {
    // Same "empty-text -> no click-target group" logic as the note case
    // above (see the comment in `click_targets_lyric.rs`), applied to a
    // padded blank verse row: assert the total syllable count for this verse
    // equals exactly what this fixture's real (non-padded) measures wrote
    // for it. Each verse token here (e.g. `v0s1-0`) contains a literal `-`,
    // which `tokenize_lyrics` splits into an extra sub-syllable (the `-`
    // marks a held syllable), so 2 written tokens per measure render as
    // `NOTE_TOKEN_COUNT` (4) syllable elements — matching the measure's 4
    // real notes exactly, unlike the note case there is no ×2 group
    // duplication for lyric click targets (only one group per syllable).
    const totalRealSyllables = state.measures.filter(
      (m) => m.melodyVerses && m.melodyVerses.length > verse,
    ).length
    await expect(
      page.locator(
        `[data-tag="lyric"][data-part-index="0"][data-verse="${verse}"]`,
      ),
    ).toHaveCount(totalRealSyllables * NOTE_TOKEN_COUNT, { timeout: 10_000 })
  },
)

Then("Melody's part label appears twice, once per system", async ({ page }) => {
  await expect(partLabelsFor(page, 'Melody')).toHaveCount(2, {
    timeout: 10_000,
  })
})
