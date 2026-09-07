import { expect, test } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Companion to `play-measure-audio.spec.ts` and
 * `note-range-select-highlight.spec.ts`: the play-measure button (see
 * `PlayMeasureButton.tsx`) is repurposed while a note range-select is active —
 * it switches to a "Selection" label and, when clicked, plays only the
 * range-selected notes (`useMeasureAudioPlayback.playNoteSelection`) instead
 * of the measure(s) under the cursor. This exercises the real playback path
 * end to end, not just the label swap.
 *
 * Self-contained source (not a demo file) with a generous "max measures per
 * system" and four single-beat notes in one measure, so all four note
 * click-targets render side by side in one row within the viewport.
 */
const rangeTestSource = [
  '# metadata',
  'title = "note range-select test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2 3 4', // measure 0
].join('\n')

Given(
  'a single-measure four-note range-select test score is loaded with the disk cache workaround',
  async ({ page, focusEditor }) => {
    test.setTimeout(75_000)

    await page.addInitScript((source) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'note-range-test.jianpu',
          userFiles: { 'note-range-test.jianpu': source },
          bin: {},
          fileIds: { 'note-range-test.jianpu': 'note-range-test-id-001' },
        }),
      )
    }, rangeTestSource)

    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })

    // Wait for the SVG preview to render note click targets for measure 0.
    await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
      timeout: 10_000,
    })

    await focusEditor()
  },
)

function noteClickTargets(page: import('@playwright/test').Page) {
  return page.locator('rect[data-variant="note-click-target-rect"]')
}

Then(
  'all four note click-targets are rendered in the measure',
  async ({ page }) => {
    await expect(noteClickTargets(page)).toHaveCount(4, { timeout: 10_000 })
  },
)

Then(
  'the play-measure button label reflects the measure under the cursor',
  async ({ page }) => {
    // Before any note range-select, the button reflects the measure under
    // the cursor, not a selection.
    const playBtn = page.locator('button.play-measure-btn')
    await expect(playBtn).toHaveText(/Measure/, { timeout: 5_000 })
  },
)

When('I click-and-click select the first three notes in the measure', async ({ page }) => {
  const noteRects = noteClickTargets(page)
  // Click-and-click sweep a marquee across the first three notes.
  const box0 = await stableBoundingBox(noteRects.nth(0))
  const box2 = await stableBoundingBox(noteRects.nth(2))
  if (!box0 || !box2) {
    throw new Error(
      'Could not get bounding boxes for notes 0 and 2. ' +
        'Ensure the SVG preview has rendered.',
    )
  }
  await clickAndClickSelect(
    page,
    box0.x + box0.width / 2,
    box0.y + box0.height / 2,
    box2.x + box2.width / 2,
    box2.y + box2.height / 2,
  )
})

Then(
  'the play-measure button label switches to Selection',
  async ({ page }) => {
    // The button is repurposed: label switches to "Selection".
    const playBtn = page.locator('button.play-measure-btn')
    await expect(playBtn).toHaveText(/Selection/, { timeout: 5_000 })
  },
)

Then(
  'the play-selection button becomes enabled once the soundfont loads',
  async ({ page }) => {
    const playBtn = page.locator('button.play-measure-btn')
    // The button stays disabled until the soundfont (a real ~30 MB asset)
    // finishes loading; wait for that instead of asserting a fixed delay.
    await expect(playBtn).toBeEnabled({ timeout: 30_000 })
  },
)

When('I click the play-selection button', async ({ page }) => {
  const playBtn = page.locator('button.play-measure-btn')
  await playBtn.click()
})

Then(
  'the play-selection button shows the playing state while still labeled Selection',
  async ({ page }) => {
    const playBtn = page.locator('button.play-measure-btn')
    // Playback engaged: button switches to the pause/playing variant, still
    // labeled "Selection".
    await expect(playBtn).toHaveClass(/play-measure-btn--playing/, {
      timeout: 5_000,
    })
    await expect(playBtn).toHaveText(/Selection/)
  },
)

Then(
  'the play-selection button eventually stops showing the playing state',
  async ({ page }) => {
    const playBtn = page.locator('button.play-measure-btn')
    // The selection is short — playback should finish and the button should
    // revert to its normal (non-playing) state on its own. Generous timeout:
    // real-time `<audio>` playback duration is sensitive to CPU
    // throttling/contention in sandboxed/CI runners, so wall-clock completion
    // can lag well past the audio's nominal duration (see FLAKY_TESTS.md).
    await expect(playBtn).not.toHaveClass(/play-measure-btn--playing/, {
      timeout: 30_000,
    })
  },
)
