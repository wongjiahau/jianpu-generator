import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then } from './fixtures'

/**
 * Regression test for the bug fixed by splitting `PlaybackCursorTarget`'s
 * `row_end` into `row_end` (playback-cursor rect, absorbs a following lyric
 * verse row) and `click_row_end` (note's own click/selection target, never
 * absorbs it) — see `resolve_note_click_target` in
 * `src/coordinate_resolver/highlights.rs` and
 * `note_click_target_does_not_extend_over_its_lyric_verse_row` in
 * `src/grid_layout/tests_playback_cursor.rs`.
 *
 * Before the fix, a note's `NoteClickTarget` rect (`data-tag="note"`,
 * `rect[data-variant="note-click-target-rect"]`) reused the same row range as
 * the playback cursor rect, which is deliberately widened to cover the
 * note's lyric verse row(s) so the "now playing" highlight visually covers
 * the lyric text too. That meant hovering/selecting a note with a lyric
 * syllable underneath drew a selection box tall enough to also cover the
 * lyric text — this spec asserts the note's own click-target rect stops
 * above the lyric row instead.
 */
const source = [
  '# metadata',
  'title = "note click target lyric row test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0 — line 9
  'do re mi fa', // verse 0 — line 10
].join('\n')

Given(
  'the note click target lyric row test fixture is loaded and lyrics have rendered',
  async ({ page }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'note-click-target-lyric-row-test.jianpu',
          userFiles: { 'note-click-target-lyric-row-test.jianpu': src },
          bin: {},
          fileIds: {
            'note-click-target-lyric-row-test.jianpu':
              'note-click-target-lyric-row-test-id-001',
          },
        }),
      )
    }, source)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
      timeout: 10_000,
    })
    await expect(page.locator('[data-tag="lyric"]')).toHaveCount(4, {
      timeout: 10_000,
    })
  },
)

Then(
  "note 0's click-target rect ends at or above its lyric row's top edge",
  async ({ page }) => {
    const noteClickRect = page
      .locator('[data-tag="note"][data-note-id="0"]')
      .locator('rect[data-variant="note-click-target-rect"]')
    const lyricRect = page
      .locator('[data-tag="lyric"][data-note-id="0"][data-verse="0"]')
      .locator('rect')

    const noteBox = await stableBoundingBox(noteClickRect)
    const lyricBox = await stableBoundingBox(lyricRect)
    expect(noteBox).not.toBeNull()
    expect(lyricBox).not.toBeNull()
    if (!noteBox || !lyricBox) return

    // The note's own click-target rect must end at or above the lyric row's
    // top edge — it must never extend down into (let alone past) the lyric
    // row it sits above.
    expect(noteBox.y + noteBox.height).toBeLessThanOrEqual(lyricBox.y + 0.5)
  },
)
