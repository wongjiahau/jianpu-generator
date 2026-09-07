import { expect, test } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

const PLAYBACK_CURSOR_FILL = 'rgba(220,38,38,0.25)'

/**
 * Regression test: hiding a part can turn its now-silent siblings' leading
 * measures into an all-rest run, which the renderer collapses into a single
 * multi-measure-rest bar (see "Multi-measure rests" in syntax.md).
 *
 * Root cause (fixed): `note_timings_seconds`/`note_timings_seconds_for_range`
 * (see `src/midi/timing_note_timings.rs`) used to build their
 * `written_blocks` by compiling the *full, unfiltered* score, so they only
 * recognised a merged rest run when the source score itself was all-rest
 * across every part. The rendered SVG instead compiles the *track-filtered*
 * score (`apply_track_filter`), so a run that's only all-rest once a part is
 * hidden merged there but not in the timing engine's block structure. The two
 * disagreed on how many blocks/note_ids that span produced, so
 * `usePlaybackCursor`'s DOM lookup (keyed by `note_id`, see
 * `usePlaybackCursor.ts`) permanently got stuck on the merged rest's
 * highlight and never advanced to the real notes that followed. Fixed by
 * threading a `visible_tracks` parameter through `note_timings_seconds`/
 * `note_timings_seconds_for_range`/`note_timings_seconds_for_literal_range`,
 * applied *before* `note_id_lookup`/`written_blocks` are built (see
 * `visible_score` in `timing_note_timings.rs`), so the timing engine's block
 * structure always matches whatever's actually rendered.
 *
 * Fixture: two parts, Melody / Harmony. Harmony alone has notes in measures
 * 0-1; Melody is implicitly resting there and first plays in measure 2.
 * Hiding Harmony turns measures 0-1 into an all-rest run for Melody, which
 * collapses into one multi-measure-rest bar. Slow tempo (bpm=60) gives an
 * 8-second rest window before Melody's first real note. The merged rest is
 * itself a sounding entry (`Tag::Note` covers "the sounding note/rest a
 * PlaybackCursorRect sits behind" — see `renderer/new_types.rs`), so it
 * correctly receives the playback cursor highlight for that whole window;
 * the regression was the cursor staying stuck there forever afterward
 * instead of advancing to Melody's real notes in measure 2.
 */
const source = [
  '# metadata',
  'title = "playback cursor rest collapse hidden part test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  'Harmony [H] = notes',
  '',
  '# score',
  'bpm=60 key=C4 time=4/4',
  '[H] 1 2 3 4', // measure 0: Harmony plays, Melody implicitly rests
  '',
  "[H] 5 6 7 1'", // measure 1: Harmony plays, Melody implicitly rests
  '',
  '[M] 1 2 3 4', // measure 2: Melody plays, Harmony implicitly rests
].join('\n')

async function loadFixture(page: import('@playwright/test').Page) {
  await page.addInitScript((src) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'playback-cursor-rest-collapse-hidden-part-test.jianpu',
        userFiles: {
          'playback-cursor-rest-collapse-hidden-part-test.jianpu': src,
        },
        bin: {},
        fileIds: {
          'playback-cursor-rest-collapse-hidden-part-test.jianpu':
            'playback-cursor-rest-collapse-hidden-part-test-id-001',
        },
      }),
    )
  }, source)
}

Given(
  'the playback-cursor rest-collapse hidden-part test fixture is loaded',
  async ({ page }) => {
    test.setTimeout(75_000)

    await loadFixture(page)
    await page.goto('/')

    await page.waitForSelector('button.play-all-btn', { timeout: 15_000 })
    await page.waitForSelector('[data-tag="measure"][data-measure-index="2"]', {
      timeout: 10_000,
    })
  },
)

When(
  'I hide the Harmony part, as seen in playback cursor rest collapse',
  async ({ page }) => {
    const harmonyPill = page.locator('.part-toggle-pill').filter({
      has: page.locator('.part-toggle-abbr', { hasText: /^H$/ }),
    })
    await harmonyPill.locator('.part-toggle-segment--eye').click()
    // Give the debounced render round-trip time to catch up with the new
    // enabledTracks (and re-collapse the now-all-rest measures) before
    // asserting on the merged bar.
    await page.waitForTimeout(2000)
  },
)

Then(
  'measures 0 and 1 render as a single multi-measure-rest for Melody',
  async ({ page }) => {
    // The measure click-target's own `data-measure-index-end` differs from
    // its `data-measure-index` only for a merged multi-measure rest (see
    // `Tag::Measure` in `renderer/new_types.rs`), so this is the DOM's own
    // signal that measures 0-1 collapsed into one bar.
    await expect(
      page.locator(
        '[data-tag="measure"][data-measure-index="0"][data-measure-index-end="1"]',
      ),
    ).toHaveCount(1, { timeout: 10_000 })
    // Melody now has exactly 5 rendered note/rest cells: the merged rest
    // (spanning measures 0-1) plus measure 2's 4 notes — not 6, which would
    // mean the merge never happened.
    await expect(
      page.locator('rect[data-variant="note-click-target-rect"]'),
    ).toHaveCount(5)
  },
)

When(
  'I click the Play All button, as seen in playback cursor rest collapse',
  async ({ page }) => {
    const playAllBtn = page.locator('button.play-all-btn')
    await expect(playAllBtn).toBeEnabled({ timeout: 30_000 })
    await playAllBtn.click()
  },
)

Then(
  'the merged multi-measure-rest shows the playback cursor highlight shortly after playback starts',
  async ({ page }) => {
    const playAllBtn = page.locator('button.play-all-btn')
    // Confirms audio has actually started (not just that we clicked play).
    await expect(playAllBtn).toHaveClass(/play-all-btn--playing/, {
      timeout: 15_000,
    })

    // The merged rest (spanning measures 0-1) is itself a sounding entry —
    // `Tag::Note` covers "the sounding note/rest a PlaybackCursorRect sits
    // behind" (see `renderer/new_types.rs`) — so it correctly receives the
    // playback cursor highlight for the whole 8-second rest window, same as
    // any other note. Find it by its bounding box falling inside measures
    // 0-1's merged bar, the same way the later steps locate measure 2's
    // first note by position rather than assuming DOM order.
    const mergedRestBox = await stableBoundingBox(
      page
        .locator(
          '[data-tag="measure"][data-measure-index="0"][data-measure-index-end="1"]',
        )
        .first(),
    )
    if (!mergedRestBox) {
      throw new Error('Could not get bounding box for the merged rest bar.')
    }
    const noteRects = page.locator(
      'rect[data-variant="note-click-target-rect"]',
    )
    const rectCount = await noteRects.count()
    let noteId: string | null = null
    let partIndex: string | null = null
    for (let i = 0; i < rectCount; i += 1) {
      const rect = noteRects.nth(i)
      const box = await stableBoundingBox(rect)
      if (
        box &&
        box.x >= mergedRestBox.x - 1 &&
        box.x < mergedRestBox.x + mergedRestBox.width
      ) {
        const group = rect.locator('xpath=..')
        noteId = await group.getAttribute('data-note-id')
        partIndex = await group.getAttribute('data-part-index')
        break
      }
    }
    if (noteId === null || partIndex === null) {
      throw new Error('Could not find the merged rest note group.')
    }

    await expect(
      page.locator(
        `[data-tag="note"][data-part-index="${partIndex}"][data-note-id="${noteId}"] rect[data-variant="playback-cursor-rect"]`,
      ),
    ).toHaveAttribute('fill', PLAYBACK_CURSOR_FILL, { timeout: 5_000 })
    // Nothing else should be highlighted yet — only the merged rest, since
    // we're still at the very start of the 8-second all-rest run.
    await expect(
      page.locator(
        `rect[data-variant="playback-cursor-rect"][fill="${PLAYBACK_CURSOR_FILL}"]`,
      ),
    ).toHaveCount(1)
  },
)

Then(
  "Melody's first note in measure 2 shows the playback cursor highlight",
  async ({ page }) => {
    const measure2Box = await stableBoundingBox(
      page.locator('[data-tag="measure"][data-measure-index="2"]').first(),
    )
    if (!measure2Box)
      throw new Error('Could not get bounding box for measure 2.')

    // Only Melody is visible (Harmony is hidden), so every rendered note
    // click-target belongs to Melody — the merged rest (spanning measures
    // 0-1) plus measure 2's four notes. Pick the one whose x falls inside
    // measure 2's own bounds rather than assuming DOM order.
    const noteRects = page.locator(
      'rect[data-variant="note-click-target-rect"]',
    )
    const rectCount = await noteRects.count()
    let noteId: string | null = null
    let partIndex: string | null = null
    for (let i = 0; i < rectCount; i += 1) {
      const rect = noteRects.nth(i)
      const box = await stableBoundingBox(rect)
      if (
        box &&
        box.x >= measure2Box.x - 1 &&
        box.x < measure2Box.x + measure2Box.width
      ) {
        const group = rect.locator('xpath=..')
        noteId = await group.getAttribute('data-note-id')
        partIndex = await group.getAttribute('data-part-index')
        break
      }
    }
    if (noteId === null || partIndex === null) {
      throw new Error('Could not find a note inside measure 2.')
    }

    // Re-query by the note's own identity (not DOM adjacency): the
    // `playback-cursor-rect` for a given `(part-index, note-id)` lives in a
    // separate rendered layer from the `note-click-target-rect`'s own group,
    // even though both carry the same `data-tag="note"` attributes — see
    // `usePlaybackCursor.ts`'s own lookup, which does the same thing.
    await expect(
      page.locator(
        `[data-tag="note"][data-part-index="${partIndex}"][data-note-id="${noteId}"] rect[data-variant="playback-cursor-rect"]`,
      ),
    ).toHaveAttribute('fill', PLAYBACK_CURSOR_FILL, { timeout: 20_000 })
  },
)

Then(
  "only Melody's first note in measure 2 shows the playback cursor highlight",
  async ({ page }) => {
    await expect(
      page.locator(
        `rect[data-variant="playback-cursor-rect"][fill="${PLAYBACK_CURSOR_FILL}"]`,
      ),
    ).toHaveCount(1)
  },
)
