import { expect } from '@playwright/test'
import { AfterScenario, Given, Then, When } from './fixtures'
import {
  SYNCED_FILENAME,
  SYNCED_SOURCE,
  seedFileStore,
  syncedShareButtonState as state,
} from './synced-share-button-state'

// Mirrors `useStorageBackend.ts`'s `AUTOSAVE_DEBOUNCE_MS`, which
// `useSyncedShareOwner.ts`'s `broadcastContent` also debounces at. Not imported
// directly — that module transitively pulls in `fileStore.ts`'s Vite-only
// `?raw` import, which Playwright's test loader can't resolve (see the same
// note in `autosave-github.steps.ts`).
const AUTOSAVE_DEBOUNCE_MS = 20_000

// `viewerContext` comes from `browser.newContext()`, which — unlike the
// per-test `context`/`page` fixtures — Playwright never closes on its own,
// so every scenario that opens one must close it itself once done. Doing
// that here (rather than in whichever assertion step happens to run last)
// means it's not tied to a particular scenario's step order.
AfterScenario(async () => {
  if (state.viewerContext) {
    await state.viewerContext.close()
    state.viewerContext = undefined
  }
})

Given('clipboard permissions are granted', async ({ context }) => {
  state.syncedShareLink = undefined
  state.originalSyncedLink = undefined
  state.viewerPage = undefined
  state.lateViewerPage = undefined
  state.viewerContext = undefined
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
})

Given('the file store is seeded with the synced score', async ({ page }) => {
  await seedFileStore(page, SYNCED_FILENAME, SYNCED_SOURCE)
})

Given(
  'the file store is seeded with a multi-measure synced range-select score',
  async ({ page }) => {
    const rangeFilename = 'synced-range-test.jianpu'
    const rangeSource = [
      '# metadata',
      'title = "Synced Range-Select Score"',
      'max_measures_per_system = 48',
      '',
      '# parts',
      'Melody [M] = notes',
      '',
      '# score',
      '[M] 1_ 1_ 1= 1= 1= 1= 1 -',
      '',
      '[M] 1. 2_ 1_. 2= 0',
      '',
      '[M] 1 - - -',
    ].join('\n')
    await seedFileStore(page, rangeFilename, rangeSource, 'synced-range-test-id')
  },
)

Given(
  'the file store is seeded with a two-section synced score',
  async ({ page }) => {
    // Mirrors `section-jump-select.steps.ts`'s two-section fixture, but with
    // `[M]`-prefixed lines (not the bare `M = notes` shorthand) so each note
    // gets its own click-target rect — needed here so a bar-line tap
    // actually paints a per-note highlight to prove stale afterward (see
    // this hook's own doc comment: the bare shorthand renders no individual
    // note click targets at all, so `applyPersistedNoteHighlights` would
    // have nothing to flag either way, silently passing regardless of the
    // bug this scenario guards against).
    const sectionFilename = 'synced-section-test.jianpu'
    const sectionSource = [
      '# metadata',
      'title = "Synced Section Score"',
      '',
      '# parts',
      'Melody [M] = notes',
      '',
      '# score',
      'time=4/4 key=C4 bpm=120 label="A"',
      '[M] 1 2 3 4',
      '',
      "[M] 5 6 7 1'",
      '',
      'label="B"',
      "[M] 1' 7 6 5",
      '',
      '[M] 4 3 2 1',
    ].join('\n')
    await seedFileStore(
      page,
      sectionFilename,
      sectionSource,
      'synced-section-test-id',
    )
  },
)

Given('local storage is cleared', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.clear()
  })
})

When(
  'the owner loads the app and clicks {string}',
  async ({ page }, label: string) => {
    expect(label).toBe('Sync')
    await page.goto('/')
    await page.getByTestId('synced-share-button').click()
  },
)

Then('a sync-link-copied toast is shown', async ({ page }) => {
  await expect(page.getByTestId('synced-share-link-copied-toast')).toBeVisible()
  state.syncedShareLink = await page.evaluate(async () => {
    return navigator.clipboard.readText()
  })
  if (state.originalSyncedLink === undefined) {
    state.originalSyncedLink = state.syncedShareLink
  }
})

When(
  'a viewer opens the copied sync link in a new page',
  async ({ context }) => {
    if (!state.syncedShareLink)
      throw new Error('syncedShareLink was not captured yet')
    state.viewerPage = await context.newPage()
    await state.viewerPage.goto(state.syncedShareLink)

    // No edit was made on the owner's side — the share's initial doc must
    // still arrive from that first fetch.
    await state.viewerPage.waitForSelector('.preview-page', {
      timeout: 15_000,
    })
  },
)

Then("the viewer's preview contains {string}", async ({}, text: string) => {
  if (!state.viewerPage) throw new Error('viewerPage was not opened yet')
  const previewContent = await state.viewerPage
    .locator('.preview-page')
    .first()
    .innerHTML()
  expect(previewContent).toContain(text)
})

Then(
  'the copied sync link contains the filename as a human-readable suffix',
  async () => {
    if (!state.syncedShareLink)
      throw new Error('syncedShareLink was not captured yet')
    expect(state.syncedShareLink).toContain(
      `--${SYNCED_FILENAME.replace(/\.jianpu$/, '')}`,
    )
  },
)

Then("the viewer's page URL has no query string", async () => {
  if (!state.viewerPage) throw new Error('viewerPage was not opened yet')
  expect(new URL(state.viewerPage.url()).search).toEqual('')
})

Then('the copied sync link matches the synced URL hash format', async () => {
  if (!state.syncedShareLink)
    throw new Error('syncedShareLink was not captured yet')
  expect(state.syncedShareLink).toMatch(/#synced=[0-9A-Za-z_-]{11}(--.+)?$/)
})

Then('the sync button now reads {string}', async ({ page }, text: string) => {
  // Once synced, the trigger becomes a dropdown offering Copy / Stop.
  await expect(page.getByTestId('synced-share-button')).toHaveText(text)
})

When('the owner clicks the sync button again', async ({ page }) => {
  await page.getByTestId('synced-share-button').click()
})

Then(
  'the copy-sync-link and stop-sync buttons are visible',
  async ({ page }) => {
    await expect(
      page.getByTestId('copy-synced-share-link-button'),
    ).toBeVisible()
    await expect(page.getByTestId('stop-sync-button')).toBeVisible()
  },
)

When('the owner clicks the copy-sync-link button', async ({ page }) => {
  await page.getByTestId('copy-synced-share-link-button').click()
})

Then('the copied link is unchanged from before', async ({ page }) => {
  if (!state.syncedShareLink)
    throw new Error('syncedShareLink was not captured yet')
  const copiedAgain = await page.evaluate(() => navigator.clipboard.readText())
  expect(copiedAgain).toEqual(state.syncedShareLink)
})

When(
  'the owner clicks the sync button and then the stop-sync button',
  async ({ page }) => {
    await page.getByTestId('synced-share-button').click()
    await page.getByTestId('stop-sync-button').click()
  },
)

Then('the stop-sync button disappears', async ({ page }) => {
  await expect(page.getByTestId('stop-sync-button')).toHaveCount(0)
})

Then('the sync button reads {string}', async ({ page }, text: string) => {
  await expect(page.getByTestId('synced-share-button')).toHaveText(text)
})

Then('the viewer sees the preview page', async () => {
  if (!state.viewerPage) throw new Error('viewerPage was not opened yet')
  await state.viewerPage.waitForSelector('.preview-page', { timeout: 15_000 })
})

Then('the viewer sees {string}', async ({}, text: string) => {
  if (!state.viewerPage) throw new Error('viewerPage was not opened yet')
  // A viewer connected *before* the owner stops should lose the score the
  // moment the owner does.
  await expect(state.viewerPage.getByText(text)).toBeVisible()
})

Then(
  "the viewer's preview no longer contains {string}",
  async ({}, text: string) => {
    if (!state.viewerPage) throw new Error('viewerPage was not opened yet')
    await expect(state.viewerPage.locator('.preview-page')).not.toContainText(
      text,
    )
  },
)

When(
  'a late viewer opens the copied sync link in a new page',
  async ({ context }) => {
    if (!state.syncedShareLink)
      throw new Error('syncedShareLink was not captured yet')
    // A fresh viewer opening the same link after the stop must not see the
    // score either — the link doesn't quietly stay viewable forever.
    state.lateViewerPage = await context.newPage()
    await state.lateViewerPage.goto(state.syncedShareLink)
  },
)

Then('the late viewer sees {string}', async ({}, text: string) => {
  if (!state.lateViewerPage)
    throw new Error('lateViewerPage was not opened yet')
  await expect(state.lateViewerPage.getByText(text)).toBeVisible()
})

Then(
  "the late viewer's preview no longer contains {string}",
  async ({}, text: string) => {
    if (!state.lateViewerPage)
      throw new Error('lateViewerPage was not opened yet')
    await expect(
      state.lateViewerPage.locator('.preview-page'),
    ).not.toContainText(text)
  },
)

When('the owner clicks {string} again', async ({ page }, label: string) => {
  expect(label).toBe('Sync')
  // Syncing again reproduces the same link and revives the share.
  await page.getByTestId('synced-share-button').click()
})

Then(
  'the revived sync link is identical to the original link',
  async ({ page }) => {
    if (!state.originalSyncedLink)
      throw new Error('originalSyncedLink was not captured yet')
    const revivedUrl = await page.evaluate(() => navigator.clipboard.readText())
    expect(revivedUrl).toEqual(state.originalSyncedLink)
  },
)

When('the late viewer reloads the page', async () => {
  if (!state.lateViewerPage)
    throw new Error('lateViewerPage was not opened yet')
  await state.lateViewerPage.reload()
  await state.lateViewerPage.waitForSelector('.preview-page', {
    timeout: 15_000,
  })
})

Then(
  "the late viewer's preview contains {string}",
  async ({}, text: string) => {
    if (!state.lateViewerPage)
      throw new Error('lateViewerPage was not opened yet')
    const previewContent = await state.lateViewerPage
      .locator('.preview-page')
      .first()
      .innerHTML()
    expect(previewContent).toContain(text)
  },
)

When('the viewer reloads the page', async () => {
  if (!state.viewerPage) throw new Error('viewerPage was not opened yet')
  await state.viewerPage.reload()
})

// Installed on the owner's page only — the synced-share push this guards is
// entirely owner-side (`useSyncedShareOwner.ts`'s debounced `broadcastContent`), so
// there is nothing for the viewer's clock to affect.
Given('the clock is under test control', async ({ page }) => {
  await page.clock.install()
})

// Edits the owner's editor content directly via the Monaco model rather
// than typing character-by-character (as `typeAtEditorEnd` does for
// appending text) — a title change needs to replace an existing line, not
// just append after it. `setValue` still fires the model's change event, so
// this exercises the same `onChange` -> `handleSourceChange` ->
// `syncedShareOwner.broadcastContent` path a real edit would.
When(
  "the owner edits the synced score's title to {string}",
  async ({ page }, title: string) => {
    const edited = SYNCED_SOURCE.replace(
      'title = "Synced Score"',
      `title = "${title}"`,
    )
    await page.evaluate((value) => {
      const editor = (
        window as unknown as {
          monaco?: typeof import('monaco-editor')
        }
      ).monaco?.editor.getEditors()[0]
      editor?.getModel()?.setValue(value)
    }, edited)
  },
)

When("the owner's autosave debounce interval elapses", async ({ page }) => {
  await page.clock.fastForward(AUTOSAVE_DEBOUNCE_MS)
})
