import type { BrowserContext, Page } from '@playwright/test'

export const FILE_STORE_KEY = 'jianpu:files:v1'
export const SYNCED_FILENAME = 'synced-test.jianpu'
export const SYNCED_SOURCE = [
  '# metadata',
  'title = "Synced Score"',
  '',
  '# parts',
  'Melody = notes',
  '',
  '# score',
  '(time=4/4 key=C4 bpm=120)',
  '1 2 3 4',
].join('\n')

export async function seedFileStore(
  page: Page,
  filename: string,
  source: string,
  fileId = 'synced-test-id',
): Promise<void> {
  await page.addInitScript(
    ({
      key,
      filename,
      source,
      fileId,
    }: {
      key: string
      filename: string
      source: string
      fileId: string
    }) => {
      localStorage.setItem(
        key,
        JSON.stringify({
          active: filename,
          userFiles: { [filename]: source },
          bin: {},
          fileIds: { [filename]: fileId },
        }),
      )
    },
    { key: FILE_STORE_KEY, filename, source, fileId },
  )
}

// Cross-step state shared between synced-share-button.steps.ts and
// synced-share-button-viewer-range-select.steps.ts (split out of one file to stay under
// the repo's max-file-lines limit). Each scenario's first Given resets this
// so state never leaks across scenarios. `syncedShareLink` always holds the most
// recently copied link; `originalSyncedLink` is pinned to the very first link
// copied in the scenario, so later "revived link" assertions can compare a
// fresh clipboard read against it without it having been clobbered by an
// intervening copy of what should be the same link.
export interface SyncedShareButtonState {
  syncedShareLink: string | undefined
  originalSyncedLink: string | undefined
  viewerPage: Page | undefined
  lateViewerPage: Page | undefined
  viewerContext: BrowserContext | undefined
}

export const syncedShareButtonState: SyncedShareButtonState = {
  syncedShareLink: undefined,
  originalSyncedLink: undefined,
  viewerPage: undefined,
  lateViewerPage: undefined,
  viewerContext: undefined,
}
