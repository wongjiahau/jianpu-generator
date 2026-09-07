import { expect } from '@playwright/test'
import { stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

const SOURCE = [
  '# metadata',
  'title = "Test"',
  'subtitle = "Sub"',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '(bpm=120 key=C4 time=4/4)',
  '[M] 1 1 5 5',
  'twin- kle twin- kle',
].join('\n')

async function loadSource(
  page: import('@playwright/test').Page,
  source: string = SOURCE,
) {
  await page.addInitScript((src) => {
    localStorage.setItem(
      'jianpu:files:v1',
      JSON.stringify({
        active: 'test.jianpu',
        userFiles: { 'test.jianpu': src },
        bin: {},
        fileIds: { 'test.jianpu': crypto.randomUUID() },
      }),
    )
  }, source)
}

async function waitForEditor(page: import('@playwright/test').Page) {
  await page.waitForSelector('.monaco-editor .view-lines', { timeout: 30_000 })
}

async function openEditMetadataModal(page: import('@playwright/test').Page) {
  await waitForEditor(page)
  const codeLensLink = page.locator('.codelens-decoration a', {
    hasText: 'Edit Metadata',
  })
  await expect(codeLensLink).toBeVisible({ timeout: 15_000 })
  await codeLensLink.click()
  await page.getByTestId('edit-metadata-modal').waitFor({ state: 'visible' })
}

async function getEditorSource(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const editors = (
      window as unknown as {
        monaco?: {
          editor?: {
            getEditors?: () => { getValue?: () => string }[]
          }
        }
      }
    ).monaco?.editor?.getEditors?.()
    return editors?.[0]?.getValue?.() ?? ''
  })
}

async function getStoredSource(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const raw = localStorage.getItem('jianpu:files:v1')
    if (!raw) return ''
    const store = JSON.parse(raw) as {
      active: string
      userFiles: Record<string, string>
    }
    return store.userFiles[store.active] ?? ''
  })
}

function editMetadataModal(page: import('@playwright/test').Page) {
  return page.getByTestId('edit-metadata-modal')
}

Given('the edit-metadata-modal test fixture is loaded', async ({ page }) => {
  await loadSource(page)
  await page.goto('/')
})

Given(
  'the edit-metadata-modal test fixture is loaded with viewport {int} by {int}',
  async ({ page }, width: number, height: number) => {
    await loadSource(page)
    await page.setViewportSize({ width, height })
    await page.goto('/')
  },
)

Given(
  'a jianpu score with 40 measures and row_height 200 is loaded, and the viewport is 1400 by 900',
  async ({ page }) => {
    const manyMeasures = Array.from({ length: 40 }, () => '1 1 5 5').join('\n')
    const source = [
      '# metadata',
      'title = "Test"',
      'row_height = 200',
      '',
      '# parts',
      'Melody [M] = notes',
      '',
      '# score',
      '(bpm=120 key=C4 time=4/4)',
      manyMeasures,
    ].join('\n')

    await loadSource(page, source)
    await page.setViewportSize({ width: 1400, height: 900 })
    await page.goto('/')
  },
)

When('I open the Edit Metadata modal', async ({ page }) => {
  await openEditMetadataModal(page)
})

Then(
  'the edit metadata modal contains {string}',
  async ({ page }, text: string) => {
    await expect(editMetadataModal(page)).toContainText(text)
  },
)

Then(
  'the first text input in the metadata modal has value {string}',
  async ({ page }, value: string) => {
    const titleInput = editMetadataModal(page)
      .locator('input[type="text"]')
      .first()
    await expect(titleInput).toHaveValue(value)
  },
)

When(
  'I fill the title field with {string}',
  async ({ page }, value: string) => {
    const modal = editMetadataModal(page)
    const titleInput = modal.locator('input[type="text"]').first()
    await titleInput.fill(value)
  },
)

When(
  'I fill the Row Height numeric field with {string}',
  async ({ page }, value: string) => {
    const modal = editMetadataModal(page)
    const rowHeightInput = modal
      .locator('tr', { hasText: 'Row Height' })
      .locator('input[type="number"]')
    await rowHeightInput.fill(value)
  },
)

// One `TextStyleRow` (see `MetadataFieldRows.tsx`) has four inline number
// inputs, one per component — each is given an `aria-label` of
// `"${rowLabel} ${componentSubLabel}"` (e.g. "Part Label Style Width",
// "Measure Number Style Font Size") specifically so a single generic step
// can target any one of them by accessible name, rather than needing a
// bespoke step (and a `tr`-text/`nth()` locator, ambiguous now that a row
// has four number inputs instead of one) per field.
When(
  'I fill the {string} field with {string}',
  async ({ page }, ariaLabel: string, value: string) => {
    const modal = editMetadataModal(page)
    const input = modal.getByLabel(ariaLabel, { exact: true })
    await input.fill(value)
  },
)

When(
  'I clear the second text input in the metadata modal',
  async ({ page }) => {
    const modal = editMetadataModal(page)
    await expect(page.locator('.monaco-editor .view-lines')).toContainText(
      'subtitle',
    )
    const subtitleInput = modal.locator('input[type="text"]').nth(1)
    await subtitleInput.fill('')
  },
)

When(
  'I uncheck the merge_duplicate_measures_across_parts checkbox',
  async ({ page }) => {
    const modal = editMetadataModal(page)
    const mergeCheckbox = modal.locator('input[type="checkbox"]').first()
    await expect(mergeCheckbox).toBeChecked()
    await mergeCheckbox.uncheck()
  },
)

When(
  'I uncheck then re-check the merge_duplicate_measures_across_parts checkbox',
  async ({ page }) => {
    const modal = editMetadataModal(page)
    const mergeCheckbox = modal.locator('input[type="checkbox"]').first()
    await mergeCheckbox.uncheck()
    await mergeCheckbox.check()
  },
)

When('I uncheck the hide_resting_parts checkbox', async ({ page }) => {
  const modal = editMetadataModal(page)
  const hideRestingCheckbox = modal.locator('input[type="checkbox"]').nth(1)
  await expect(hideRestingCheckbox).toBeChecked()
  await hideRestingCheckbox.uncheck()
})

When(
  'I uncheck then re-check the hide_resting_parts checkbox',
  async ({ page }) => {
    const modal = editMetadataModal(page)
    const hideRestingCheckbox = modal.locator('input[type="checkbox"]').nth(1)
    await hideRestingCheckbox.uncheck()
    await hideRestingCheckbox.check()
  },
)

When('I check the hide_system_dividers checkbox', async ({ page }) => {
  const modal = editMetadataModal(page)
  const hideDividersCheckbox = modal.locator('input[type="checkbox"]').last()
  await expect(hideDividersCheckbox).not.toBeChecked()
  await hideDividersCheckbox.check()
})

When(
  'I check then uncheck the hide_system_dividers checkbox',
  async ({ page }) => {
    const modal = editMetadataModal(page)
    const hideDividersCheckbox = modal.locator('input[type="checkbox"]').last()
    await hideDividersCheckbox.check()
    await hideDividersCheckbox.uncheck()
  },
)

When(
  'I fill the directive_row_offset field with {string}',
  async ({ page }, value: string) => {
    const modal = editMetadataModal(page)
    const offsetInput = modal.locator('input[type="text"]').nth(3)
    await offsetInput.fill(value)
  },
)

When('I close the metadata modal with Escape', async ({ page }) => {
  const modal = editMetadataModal(page)
  await page.keyboard.press('Escape')
  await modal.waitFor({ state: 'hidden' })
})

Then(
  'the editor source and stored source both contain {string}',
  async ({ page }, expectedLine: string) => {
    await expect.poll(getEditorSource.bind(null, page)).toContain(expectedLine)
    await expect.poll(getStoredSource.bind(null, page)).toContain(expectedLine)
  },
)

Then(
  'the editor source and stored source no longer contain {string}',
  async ({ page }, text: string) => {
    await expect.poll(getEditorSource.bind(null, page)).not.toContain(text)
    await expect.poll(getStoredSource.bind(null, page)).not.toContain(text)
  },
)

Then(
  'the metadata modal stays within the editor pane and does not cover the preview pane',
  async ({ page }) => {
    const modal = editMetadataModal(page)
    const modalBox = await stableBoundingBox(modal)
    const previewBox = await stableBoundingBox(page.locator('.pane--preview'))
    if (!modalBox || !previewBox) {
      throw new Error('expected modal and preview pane to have bounding boxes')
    }

    expect(modalBox.x + modalBox.width).toBeLessThanOrEqual(previewBox.x)
  },
)

Then('the preview pane is scrollable', async ({ page }) => {
  const previewPages = page.locator('.preview-pages')
  await expect
    .poll(async () =>
      previewPages.evaluate((el) => el.scrollHeight > el.clientHeight),
    )
    .toBe(true)
})

When(
  'I hover the preview pane and scroll the mouse wheel down by 400',
  async ({ page }) => {
    const previewPages = page.locator('.preview-pages')
    await previewPages.hover()
    await page.mouse.wheel(0, 400)
  },
)

Then('the preview pane scroll position is greater than 0', async ({ page }) => {
  const previewPages = page.locator('.preview-pages')
  await expect
    .poll(() => previewPages.evaluate((el) => el.scrollTop))
    .toBeGreaterThan(0)
})
