import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression test: a measure/bar-line click-and-click range that spans
 * multiple systems used to mis-select notes belonging to a part declared
 * *after* a currently-hidden part.
 *
 * Root cause: `noteSpans` (fetched via the `listNoteSpans` worker message,
 * see `jianpu.worker.ts`) comes from `list_note_spans_from_source`, which
 * used to compile the score with no track filter at all — so each span's
 * `sourcePartIndex` was the event's raw index into `MultiPartMeasure::parts`,
 * i.e. every declared part counted, including hidden ones.
 *
 * The rendered SVG is different: `render_svgs_with_parts`'s pipeline runs
 * `apply_track_filter` first (see `document_render.rs`/`filters.rs`), which
 * `Vec::retain`s hidden parts *out of* `measure.parts` before compiling —
 * so every part declared after a hidden one is compacted down by one index
 * in the SVG's `data-part-index` attributes.
 *
 * `usePreviewClickSelection.ts`'s 'measure' mode (`noteCellsInMeasureRange`)
 * resolves a range-select's selected cells straight from `noteSpans` and then marks
 * the matching `data-part-index`/`data-note-id` DOM groups
 * (`applyPersistedNoteHighlights`) — so once those two "part index"
 * numberings disagreed, cells resolved from `noteSpans` no longer lined up
 * with the compacted indices actually present in the DOM for any part
 * declared after the hidden one.
 *
 * Fix: `list_note_spans_from_source`/`list_lyric_spans_from_source` (and
 * their wasm bindings `list_note_spans`/`list_lyric_spans`) now take an
 * `enabled_tracks` filter and apply `apply_track_filter` themselves before
 * walking the score, so their indices always match whatever the renderer
 * used for the same `enabledTracks`. The frontend now threads the current
 * `enabledTracks` through the `listNoteSpans`/`listLyricSpans` worker
 * messages (see `useJianpuWorkerRenderRequests.ts`) instead of omitting it.
 *
 * Fixture: three parts, Melody / Harmony / Bass, two systems (one measure
 * each). Harmony (the middle part) is hidden, which compacts Bass from
 * source part-index 2 down to rendered part-index 1. Harmony has 1 note per
 * measure and Bass has 2, so — before the fix — Harmony's unfiltered
 * `sourcePartIndex:noteId` keys ("1:0") only ever accidentally collided
 * with *some* of Bass's rendered keys: enough to make system 0 (measure 0)
 * look right by coincidence, while system 1 (measure 1) had no such
 * collision and simply failed to select Bass's notes at all. That's the
 * "cross-system" symptom this guards against: the same range-select behaving
 * correctly in the system it started in and silently dropping notes in the
 * next one.
 */
const source = [
  '# metadata',
  'title = "measure range-select hidden part index mismatch"',
  'max_measures_per_system = 1',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  'Bass [B] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0 — system 0
  '[H] 5', // 1 note/measure
  '[B] 3 4', // 2 notes/measure
  '',
  '[M] 3 4', // measure 1 — system 1
  '[H] 7',
  '[B] 5 6',
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((src) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'measure-range-hidden-part-test.jianpu',
        userFiles: { 'measure-range-hidden-part-test.jianpu': src },
        bin: {},
        fileIds: {
          'measure-range-hidden-part-test.jianpu':
            'measure-range-hidden-part-test-id-001',
        },
      }),
    )
  }, source)
}

Given(
  'the measure-range-select hidden-part test fixture is loaded',
  async ({ page }) => {
    await loadFixture(page)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="measure"][data-measure-index="1"]', {
      timeout: 10_000,
    })
  },
)

When('I hide the Harmony part', async ({ page }) => {
  // Hide Harmony — the middle part — so Bass compacts from rendered
  // part-index 2 down to 1.
  const harmonyPill = page.locator('.part-toggle-pill').filter({
    has: page.locator('.part-toggle-abbr', { hasText: /^H$/ }),
  })
  await harmonyPill.locator('.part-toggle-segment--eye').click()
})

Then('{int} notes render across both measures', async ({ page }, count) => {
  const noteRects = page.locator('rect[data-variant="note-click-target-rect"]')
  // Melody (2/measure) + Bass (2/measure), 2 measures = 8 rendered notes.
  await expect(noteRects).toHaveCount(count, { timeout: 10_000 })
  // Give the debounced listNoteSpans worker round-trip time to catch up
  // with the new enabledTracks before range-selecting.
  await page.waitForTimeout(2000)
})

When(
  "I Cmd\\/Ctrl-click-and-click from measure 0's left bar line into measure 1's interior",
  async ({ page }) => {
    const measures = page.locator('[data-tag="measure"]')
    const firstBox = await stableBoundingBox(measures.nth(0))
    const lastMeasureIndex = (await measures.count()) - 1
    const lastBox = await stableBoundingBox(measures.nth(lastMeasureIndex))
    if (!firstBox || !lastBox) {
      throw new Error('Could not get bounding boxes for measures 0 and 1.')
    }

    // Click-and-click starting exactly on measure 0's left bar line and
    // ending in measure 1's interior — a measure-mode selection spanning
    // both systems. Held under Cmd/Ctrl, the only way to reach 'measure'
    // mode now (see `previewClickHandler.ts`'s `handlePreviewClick`).
    await page.keyboard.down('Control')
    await clickAndClickSelect(
      page,
      firstBox.x,
      firstBox.y + firstBox.height / 2,
      lastBox.x + lastBox.width / 2,
      lastBox.y + lastBox.height / 2,
    )
    await page.keyboard.up('Control')
  },
)

Then(
  '{int} notes are range-selected, as seen in measure click selects notes with hidden part',
  async ({ page }, count: number) => {
    // Every rendered note (Melody's 4 + Bass's 4) should be selected — Bass
    // is a fully visible part and both its measures sit inside the
    // range-selected range.
    await expect(
      page.locator('[data-tag="note"][data-note-range-selected]'),
    ).toHaveCount(count)
  },
)

Then(
  "Bass's measure-1 notes with ids {int} and {int} at part-index 1 are range-selected",
  async ({ page }, a: number, b: number) => {
    // In particular, Bass's system-1 (measure 1) notes — note ids 2 and 3,
    // since each part's note-id counter runs across the whole score rather
    // than resetting per measure — must be selected. This is the pair the
    // index mismatch silently drops (Harmony's unfiltered spans only ever
    // collide with Bass's measure-0 ids 0/1, not measure-1's 2/3).
    for (const noteId of [a, b]) {
      await expect(
        page.locator(
          `[data-tag="note"][data-note-range-selected][data-part-index="1"][data-note-id="${noteId}"]`,
        ),
      ).toHaveCount(1)
    }
  },
)
