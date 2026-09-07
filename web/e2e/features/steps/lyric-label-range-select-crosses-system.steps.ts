import { expect } from '@playwright/test'
import { clickAndClickSelect, stableBoundingBox } from '../../rangeSelectHelpers'
import { Given, Then, When } from './fixtures'

/**
 * Regression fixture for the click-and-click *lyric-label* range gesture not
 * spanning a system boundary — the lyric-label mirror of
 * `note-range-select-crosses-system.feature`. Range resolution shouldn't
 * know the concept of a "system" at all: a plain click-and-click from a
 * verse label in one system to the same verse's label in a later system
 * should select every syllable that verse sings in between, exactly like
 * `Note ↔ Note` already does across a system boundary (see
 * `resolve_selection_range_response` in `selection_range.rs`), with no
 * Cmd/Ctrl modifier required. Today `LyricLabel ↔ LyricLabel` is only
 * ID-resolved for a same-system pair (guarded on matching
 * `measure_index_start`/`measure_index_end`); a cross-system pair falls
 * through to the pixel-marquee fallback (`lyricLabelsInMarquee`), which
 * restricts to the *anchor's own system* — so today's plain click-and-click
 * only selects the first system's syllables, dropping the second system's
 * entirely.
 *
 * `break` forces measure 1 onto its own system regardless of
 * `max_measures_per_system`, so this fixture needs no filler content to
 * reproduce the boundary.
 */
const source = [
  '# metadata',
  'title = "cross system lyric label range test"',
  'max_measures_per_system = 48',
  '',
  '# parts',
  'Melody [M] = notes',
  '',
  '# score',
  '[M] 1 2', // measure 0 — system 0
  'do re', // verse 0
  '',
  'break',
  '[M] 3 4', // measure 1 — system 1
  'la ti', // verse 0
].join('\n')

function verseLabelInSystem(
  page: import('@playwright/test').Page,
  verse: number,
  measureIndexStart: number,
) {
  return page.locator(
    `[data-tag="lyric-label"][data-part-index="0"][data-verse="${verse}"][data-measure-index-start="${measureIndexStart}"]`,
  )
}

Given(
  'the cross-system lyric-label range-selection fixture is loaded and labels have rendered',
  async ({ page }) => {
    await page.addInitScript((src) => {
      localStorage.setItem(
        'jianpu:files:v1',
        JSON.stringify({
          active: 'cross-system-lyric-label-range-test.jianpu',
          userFiles: { 'cross-system-lyric-label-range-test.jianpu': src },
          bin: {},
          fileIds: {
            'cross-system-lyric-label-range-test.jianpu':
              'cross-system-lyric-label-range-test-id-001',
          },
        }),
      )
    }, source)
    await page.goto('/')

    await page.waitForSelector('[data-testid="play-measure-button"]', {
      timeout: 15_000,
    })
    // System 0's verse-0 label sits at measure-index-start 0, system 1's at
    // measure-index-start 1 — see `verseLabelInSystem`.
    await page.waitForSelector(
      '[data-tag="lyric-label"][data-part-index="0"][data-verse="0"][data-measure-index-start="0"]',
      { timeout: 10_000 },
    )
    await page.waitForSelector(
      '[data-tag="lyric-label"][data-part-index="0"][data-verse="0"][data-measure-index-start="1"]',
      { timeout: 10_000 },
    )
  },
)

When(
  'I click-and-click select the verse {int} label in system 0 then the verse {int} label in system 1',
  async ({ page }, fromVerse: number, toVerse: number) => {
    const fromLabel = verseLabelInSystem(page, fromVerse, 0)
    const toLabel = verseLabelInSystem(page, toVerse, 1)
    await expect(fromLabel).toBeVisible({ timeout: 5_000 })
    await expect(toLabel).toBeVisible({ timeout: 5_000 })

    const fromBox = await stableBoundingBox(fromLabel)
    const toBox = await stableBoundingBox(toLabel)
    if (!fromBox || !toBox) {
      throw new Error(
        'Could not get bounding boxes for the cross-system verse labels.',
      )
    }

    await clickAndClickSelect(
      page,
      fromBox.x + fromBox.width / 2,
      fromBox.y + fromBox.height / 2,
      toBox.x + toBox.width / 2,
      toBox.y + toBox.height / 2,
    )
  },
)

Then(
  "verse 0's 4 syllables across both systems are all range-selected",
  async ({ page }) => {
    // Verse 0 sounds 4 syllables total across both systems ("do re" + "la
    // ti") — the range should cover every one of them, not just the
    // anchor's own system.
    const highlightedLyrics = page.locator(
      '[data-tag="lyric"][data-lyric-range-selected][data-verse="0"]',
    )
    await expect(highlightedLyrics).toHaveCount(4)
  },
)
