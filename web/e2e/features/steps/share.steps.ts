import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { fileSwitcherTrigger, openFileActions } from '../../fileSwitcherHelpers'
import { encodeShareHashOnPage, gotoShareUrl } from '../../shareUrlHelper'
import { Given, Then, When } from './fixtures'

const FILE_STORE_KEY = 'jianpu:files:v1'
const SHARED_FILENAME = 'shared-test.jianpu'
const SHARED_SOURCE = [
  '# metadata',
  'title = "Shared Score"',
  '',
  '# parts',
  'Melody = notes',
  '',
  '# score',
  '(time=4/4 key=C4 bpm=120)',
  '1 2 3 4',
].join('\n')

let lastShareUrl: string | undefined

Given('local storage is cleared, as seen in share', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.clear()
  })
})

When(
  'I open the share URL for {string}',
  async ({ page }, filename: string) => {
    expect(filename).toBe(SHARED_FILENAME)
    await gotoShareUrl(page, SHARED_FILENAME, SHARED_SOURCE)
  },
)

When(
  'I open the share URL for {string} again',
  async ({ page }, filename: string) => {
    expect(filename).toBe(SHARED_FILENAME)
    await gotoShareUrl(page, SHARED_FILENAME, SHARED_SOURCE)
  },
)

When('I reload the page', async ({ page }) => {
  await page.reload()
})

When(
  'I navigate to a legacy uncompressed share link for {string}',
  async ({ page }, filename: string) => {
    expect(filename).toBe(SHARED_FILENAME)
    const legacyPayload = encodeURIComponent(
      JSON.stringify({ filename: SHARED_FILENAME, content: SHARED_SOURCE }),
    )
    await page.goto(`http://localhost:5173/#share=${legacyPayload}`)
  },
)

When('I click {string}', async ({ page }, buttonName: string) => {
  await page.getByRole('button', { name: buttonName }).click()
})

When('I click the pane-divider toggle, as seen in share', async ({ page }) => {
  await page.locator('.pane-divider-toggle').click()
})

Given(
  'clipboard permissions are granted, as seen in share',
  async ({ context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  },
)

Given(
  'a user file {string} is seeded in local storage',
  async ({ page }, filename: string) => {
    expect(filename).toBe(SHARED_FILENAME)
    await page.addInitScript(
      ({
        key,
        filename,
        source,
      }: {
        key: string
        filename: string
        source: string
      }) => {
        localStorage.setItem(
          key,
          JSON.stringify({
            active: filename,
            userFiles: { [filename]: source },
            bin: {},
            fileIds: { [filename]: 'share-test-id' },
          }),
        )
      },
      { key: FILE_STORE_KEY, filename: SHARED_FILENAME, source: SHARED_SOURCE },
    )
  },
)

When('the app loads, as seen in share', async ({ page }) => {
  await page.goto('/')
})

When(
  'I open the file actions menu and click the share button',
  async ({ page }) => {
    await openFileActions(page)
    await page.getByTestId('share-button').click()
  },
)

When('I navigate fresh to the copied share URL', async ({ page }) => {
  if (!lastShareUrl) {
    throw new Error('lastShareUrl was not captured before this step')
  }
  await page.goto('/')
  await page.evaluate(() => localStorage.clear())
  // A hash-only change from the current document is a same-document
  // navigation and won't remount the app, unlike a real recipient opening
  // the link fresh — force a full navigation via a blank interstitial page.
  await page.goto('about:blank')
  await page.goto(lastShareUrl)
})

Then('the shared preview banner is visible', async ({ page }) => {
  await expect(page.locator('.shared-preview-banner')).toBeVisible()
})

Then('the shared preview banner is gone', async ({ page }) => {
  await expect(page.locator('.shared-preview-banner')).toHaveCount(0)
})

Then('the file switcher is hidden entirely', async ({ page }) => {
  // The file switcher is hidden entirely while previewing a shared score.
  await expect(fileSwitcherTrigger(page)).toHaveCount(0)
})

Then('the preview contains {string}', async ({ page }, text: string) => {
  await page.waitForSelector('.preview-page', { timeout: 15_000 })
  const previewContent = await page.locator('.preview-page').first().innerHTML()
  expect(previewContent).toContain(text)
})

Then('the file switcher shows {string}', async ({ page }, filename: string) => {
  await expect(fileSwitcherTrigger(page)).toContainText(filename)
})

Then(
  'the file switcher no longer shows {string}',
  async ({ page }, filename: string) => {
    await expect(fileSwitcherTrigger(page)).not.toContainText(filename)
  },
)

Then('the editor pane is collapsed, as seen in share', async ({ page }) => {
  await expect(page.locator('.pane--editor')).toHaveClass(
    /pane--editor-collapsed/,
  )
})

Then('the editor pane is expanded, as seen in share', async ({ page }) => {
  await expect(page.locator('.pane--editor')).not.toHaveClass(
    /pane--editor-collapsed/,
  )
})

Then('the pane-divider toggle is hidden', async ({ page }) => {
  await expect(page.locator('.pane-divider-toggle')).toHaveCount(0)
})

Then('the pane-divider toggle is visible again', async ({ page }) => {
  // The toggle reappears once the shared preview is dismissed, letting the
  // user re-expand the editor pane manually.
  await expect(page.locator('.pane-divider-toggle')).toBeVisible()
})

// A self-contained single-measure score, distinct from `SHARED_SOURCE`
// above: every data line here is `[Melody]`-prefixed (see syntax.md's "Every
// data line must begin with `[Abbrev]`"), so the note actually renders
// instead of erroring out as a positional-lyrics line with no preceding
// `[Key]` line.
const NOTE_TAP_SHARED_SOURCE = [
  '# metadata',
  'title = "Shared Score"',
  '',
  '# parts',
  'Melody = notes',
  '',
  '# score',
  'time=4/4 key=C4 bpm=120',
  '[Melody] 1 2 3 4',
].join('\n')

When(
  'I open a shared preview with a valid tappable note, as seen in share',
  async ({ page }) => {
    await gotoShareUrl(page, SHARED_FILENAME, NOTE_TAP_SHARED_SOURCE)
  },
)

When('I tap the first note, as seen in share', async ({ page }) => {
  await page.waitForSelector('[data-tag="measure"][data-measure-index="0"]', {
    timeout: 15_000,
  })
  const noteRect = page
    .locator('rect[data-variant="note-click-target-rect"]')
    .first()
  await expect(noteRect).toBeVisible()
  const box = await stableBoundingBox(noteRect)
  if (!box) throw new Error('Could not get bounding box for the first note.')
  // A single click-and-click at the same point selects just that one note
  // (see `clickAndClickSelect`'s doc comment) — the regression this guards
  // against is a plain tap also painting the whole-measure amber overlay in
  // this no-mounted-editor viewer (see `fireCommit`'s and
  // `useMeasureRangeSelection`'s doc comments).
  await clickAndClickSelect(
    page,
    box.x + box.width / 2,
    box.y + box.height / 2,
    box.x + box.width / 2,
    box.y + box.height / 2,
  )
})

Then('the tapped note is highlighted, as seen in share', async ({ page }) => {
  await expect(
    page.locator('[data-tag="note"][data-note-range-selected]'),
  ).toHaveCount(1, { timeout: 5_000 })
})

Then(
  'the measure highlight is not shown, as seen in share',
  async ({ page }) => {
    await page.waitForTimeout(1000)
    await expect(
      page.locator('.preview-page [data-testid="measure-highlight"]'),
    ).toHaveCount(0)
  },
)

// Mirrors `section-jump-select.steps.ts`'s two-section fixture, used there
// to cover the same section-label click against a mounted editor — this
// covers the no-mounted-editor (shared-preview) counterpart, where a
// section jump has no Monaco selection to echo a highlight back from (see
// `useSectionNavigation.selectSectionRange`'s doc comment).
const TWO_SECTION_SHARED_SOURCE = [
  '# metadata',
  'title = "Shared Score"',
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

When(
  'I open a shared preview with a two-section score, as seen in share',
  async ({ page }) => {
    await gotoShareUrl(page, SHARED_FILENAME, TWO_SECTION_SHARED_SOURCE)
  },
)

When(
  'I click the section label {string} in the SVG preview, as seen in share',
  async ({ page }, label: string) => {
    const svgLabel = page
      .locator(
        `.preview-pages g[data-tag="section-label"][data-section-label="${label}"]`,
      )
      .first()
    await svgLabel.waitFor({ timeout: 15_000 })
    await svgLabel.click()
  },
)

Then(
  "section B's measures are amber-highlighted, as seen in share",
  async ({ page }) => {
    await expect(
      page.locator('.preview-page [data-testid="measure-highlight"]'),
    ).toHaveCount(2, { timeout: 5_000 })
  },
)

Then('the share button shows {string}', async ({ page }, text: string) => {
  await expect(page.getByTestId('share-button')).toHaveText(text)
})

Then(
  'the copied share URL matches the expected compressed hash for {string}',
  async ({ page }, filename: string) => {
    expect(filename).toBe(SHARED_FILENAME)
    const shareUrl = await page.evaluate(async () => {
      return navigator.clipboard.readText()
    })

    const expectedHash = await encodeShareHashOnPage(
      page,
      SHARED_FILENAME,
      SHARED_SOURCE,
    )
    expect(shareUrl).toContain(`#share=${expectedHash}`)
    lastShareUrl = shareUrl
  },
)
