import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Source with a `# sequence` section that replays "B" twice
 * (`A, B, B`), so the resolved playback order (A, B, B) diverges from
 * written order (A, B) — this is what distinguishes the sequence jump
 * toolbar (SequenceJumpToolbar) from the plain section jump toolbar
 * (SectionJumpToolbar), which only ever shows one button per written label.
 *
 * Lines (1-based):
 *   7:  # sequence
 *   8:  A, B, B
 *   10: # score
 *   11: time=4/4 key=C4 bpm=120 label="A"   ← measure 0
 *   12: 1 2 3 4
 *   14: label="B"                           ← measure 1
 *   15: 5 6 7 1'
 */
const source = [
  '# metadata',
  'title = "test"',
  '',
  '# parts',
  'M = notes',
  '',
  '# sequence',
  'A, B, B',
  '',
  '# score',
  'time=4/4 key=C4 bpm=120 label="A"',
  '1 2 3 4',
  '',
  'label="B"',
  "5 6 7 1'",
].join('\n')

// The section jump toolbar (always present when labels exist) and the
// sequence jump toolbar (only present when a `# sequence` section resolves)
// both render `button.section-jump-btn` elements inside their own
// `[role="toolbar"]`. `SequenceJumpToolbar` is mounted after
// `SectionJumpToolbar` in App.tsx, so it is the second toolbar in the DOM.
function sequenceToolbarButtons(page: import('@playwright/test').Page) {
  return page
    .locator('[role="toolbar"]')
    .nth(1)
    .locator('button.section-jump-btn')
}

Given(
  'a source with a repeating sequence {string} is loaded',
  async ({ page }, _sequence: string) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'sequence-test.jianpu',
          userFiles: { 'sequence-test.jianpu': src },
          bin: {},
          fileIds: { 'sequence-test.jianpu': crypto.randomUUID() },
        }),
      )
    }, source)

    await page.goto('/')

    // The play-from-current-measure button only renders once a sequence
    // entry is selected, so wait on a button that's always present (once
    // audio is available) to confirm the app has finished loading.
    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    // Sequence buttons appear once the worker returns resolved sequence
    // entries, which lands slightly after the play button.
    await expect(sequenceToolbarButtons(page)).toHaveCount(3, {
      timeout: 15_000,
    })
  },
)

Then(
  'the sequence toolbar shows buttons {string} in order',
  async ({ page }, labels: string) => {
    const buttons = sequenceToolbarButtons(page)
    const expected = labels.split(',').map((label) => label.trim())
    for (let i = 0; i < expected.length; i++) {
      await expect(buttons.nth(i)).toHaveText(expected[i])
    }
  },
)

Then('the play-from-current-measure button is hidden', async ({ page }) => {
  const playBtn = page.getByTestId('play-from-current-measure-button')
  await expect(playBtn).toHaveCount(0)
})

When(
  'I click the sequence toolbar button at index {int}',
  async ({ page }, index: number) => {
    await sequenceToolbarButtons(page).nth(index).click()
  },
)

Then(
  'the play-from-current-measure button aria-label says {string}',
  async ({ page }, label: string) => {
    const playBtn = page.getByTestId('play-from-current-measure-button')
    // Selecting a sequence entry updates the aria-label immediately; whether
    // the button is actually clickable also depends on the (real, ~30 MB)
    // soundfont finishing its load, which is covered separately in
    // sequence-jump-toolbar-play.spec.ts.
    await expect(playBtn).toHaveAttribute('aria-label', label)
  },
)

// The two "B" buttons (indices 1 and 2) both map to the same written measure
// (index 1), since `# sequence` just replays that span twice. Clicking one
// must highlight only that specific occurrence — proving selection is keyed
// by sequence position, not by label or by measure range.
Then(
  'sequence toolbar button {int} is highlighted as active',
  async ({ page }, index: number) => {
    await expect(sequenceToolbarButtons(page).nth(index)).toHaveClass(
      /section-jump-btn--dragging/,
    )
  },
)

Then(
  'sequence toolbar button {int} is not highlighted as active',
  async ({ page }, index: number) => {
    await expect(sequenceToolbarButtons(page).nth(index)).not.toHaveClass(
      /section-jump-btn--dragging/,
    )
  },
)

// Dragging from one entry button to another selects the merged range spanning
// both, via useSequenceNavigation's handleSequenceEntryRangeSelect — mirrors
// the drag coverage in section-jump-select.spec.ts for the (separate) section
// jump toolbar.
When(
  'I drag from sequence toolbar button {int} to sequence toolbar button {int}',
  async ({ page }, fromIndex: number, toIndex: number) => {
    const buttons = sequenceToolbarButtons(page)
    const from = buttons.nth(fromIndex)
    const to = buttons.nth(toIndex)

    const fromBox = await stableBoundingBox(from)
    const toBox = await stableBoundingBox(to)
    if (!fromBox || !toBox) {
      throw new Error(
        'Could not get bounding boxes for sequence entry buttons.',
      )
    }

    await page.mouse.move(
      fromBox.x + fromBox.width / 2,
      fromBox.y + fromBox.height / 2,
    )
    await page.mouse.down()
    await expect(from).toHaveClass(/section-jump-btn--dragging/, {
      timeout: 3_000,
    })
    await page.mouse.move(
      toBox.x + toBox.width / 2,
      toBox.y + toBox.height / 2,
      { steps: 10 },
    )
    await expect(to).toHaveClass(/section-jump-btn--dragging/, {
      timeout: 3_000,
    })
  },
)

When('I release the mouse button on the sequence toolbar', async ({ page }) => {
  // The merged range still starts at measure 0 (the "A" entry's start), so
  // playback is enabled from measure 1, and both buttons stay highlighted as
  // the active (not merely dragging) selection.
  await page.mouse.up()
})

// Touch equivalent of the drag test above. Real touchmove events (unlike
// mouse) keep their `target` pinned to whatever element the touch started on,
// so SequenceJumpToolbar resolves the button under the finger via
// `document.elementFromPoint` instead of relying on onMouseEnter — dispatch
// actual touch events (via CDP) to exercise that path directly.
When(
  'I touch-drag from sequence toolbar button {int} to sequence toolbar button {int}',
  async ({ page }, fromIndex: number, toIndex: number) => {
    const buttons = sequenceToolbarButtons(page)
    const buttonFrom = buttons.nth(fromIndex)
    const buttonTo = buttons.nth(toIndex)

    const fromBox = await stableBoundingBox(buttonFrom)
    const toBox = await stableBoundingBox(buttonTo)
    if (!fromBox || !toBox) {
      throw new Error(
        'Could not get bounding boxes for sequence entry buttons.',
      )
    }

    const fromPoint = {
      x: fromBox.x + fromBox.width / 2,
      y: fromBox.y + fromBox.height / 2,
    }
    const toPoint = {
      x: toBox.x + toBox.width / 2,
      y: toBox.y + toBox.height / 2,
    }

    const client = await page.context().newCDPSession(page)

    await client.send('Input.dispatchTouchEvent', {
      type: 'touchStart',
      touchPoints: [{ x: fromPoint.x, y: fromPoint.y }],
    })
    await expect(buttonFrom).toHaveClass(/section-jump-btn--dragging/, {
      timeout: 3_000,
    })

    const steps = 10
    for (let step = 1; step <= steps; step += 1) {
      const x = fromPoint.x + ((toPoint.x - fromPoint.x) * step) / steps
      const y = fromPoint.y + ((toPoint.y - fromPoint.y) * step) / steps
      await client.send('Input.dispatchTouchEvent', {
        type: 'touchMove',
        touchPoints: [{ x, y }],
      })
    }
    await expect(buttonTo).toHaveClass(/section-jump-btn--dragging/, {
      timeout: 3_000,
    })

    await client.send('Input.dispatchTouchEvent', {
      type: 'touchEnd',
      touchPoints: [],
    })
  },
)
