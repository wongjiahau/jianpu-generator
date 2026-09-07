import type { Locator, Page } from '@playwright/test'

/**
 * Drives the preview's click-and-click range-selection gesture (see
 * `previewClickHandler.ts`/`usePreviewClickSelection.ts`): a first click at
 * `(startX, startY)` anchors the selection, a `mousemove` to `(endX, endY)`
 * live-updates the hover preview (mirroring what a real mouse user sees
 * between the two clicks), and a second click at `(endX, endY)` resolves and
 * commits the range. Replaces the old held-button drag
 * (`mouse.down()` → `mouse.move({ steps })` → `mouse.up()`) every preview
 * drag-select e2e step used before the gesture became click-and-click.
 */
export async function clickAndClickSelect(
  page: Page,
  startX: number,
  startY: number,
  endX: number,
  endY: number,
  moveSteps = 10,
): Promise<void> {
  await page.mouse.move(startX, startY)
  await page.mouse.down()
  await page.mouse.up() // click #1 — anchors
  await page.mouse.move(endX, endY, { steps: moveSteps }) // hover preview
  await page.mouse.down()
  await page.mouse.up() // click #2 — commits
}

/**
 * Polls a locator's `boundingBox()` until three consecutive reads agree, so
 * a coordinate-based click isn't computed against a box that's still
 * settling — e.g. the anchor click's own self-commit re-render (see
 * `anchorAndCommit`'s doc comment) landing between when a step measures the
 * second endpoint's box and when it actually clicks there. Lifted out of
 * `note-range-select-crosses-page.steps.ts` (its own doc comment has the
 * fuller story on why this class of flake exists) since every click-and-click
 * step measuring a *second* endpoint's box needs the same guard, not just
 * that file's scroll-across-pages case.
 */
export async function stableBoundingBox(locator: Locator) {
  let previous = await locator.boundingBox()
  let matches = 0
  for (let i = 0; i < 30; i++) {
    await new Promise((resolve) => setTimeout(resolve, 50))
    const current = await locator.boundingBox()
    if (
      previous &&
      current &&
      previous.x === current.x &&
      previous.y === current.y
    ) {
      // Requires 3 consecutive agreeing reads, not just 2 — under heavy
      // parallel-worker CPU contention, two reads can land in a brief lull
      // between two separate layout-shifting events and falsely agree.
      matches++
      if (matches >= 2) return current
    } else {
      matches = 0
    }
    previous = current
  }
  return previous
}

/**
 * The click-and-click-select shape every label-mixed regression fixture
 * uses (see e.g. `note-partlabel-range-select.steps.ts`): clicks `from`
 * immediately (anchoring the gesture), *then* resolves `to`'s box via
 * `stableBoundingBox` — after the anchor click's own self-commit re-render
 * has had a chance to settle, not before it — and clicks there. Measuring
 * both endpoints' boxes up front (as `clickAndClickSelect` above expects)
 * risks the exact stale-coordinate race `stableBoundingBox`'s doc comment
 * describes whenever the anchor click's own re-render shifts the second
 * endpoint's position between measurement and the second click.
 */
export async function clickThenStableClick(
  page: Page,
  from: Locator,
  to: Locator,
): Promise<void> {
  const fromBox = await stableBoundingBox(from)
  if (!fromBox)
    throw new Error('Could not get a bounding box for the anchor endpoint.')
  await page.mouse.move(
    fromBox.x + fromBox.width / 2,
    fromBox.y + fromBox.height / 2,
  )
  await page.mouse.down()
  await page.mouse.up() // click #1 — anchors

  const toBox = await stableBoundingBox(to)
  if (!toBox)
    throw new Error('Could not get a bounding box for the target endpoint.')
  await page.mouse.move(toBox.x + toBox.width / 2, toBox.y + toBox.height / 2, {
    steps: 10,
  })
  await page.mouse.down()
  await page.mouse.up() // click #2 — commits
}
