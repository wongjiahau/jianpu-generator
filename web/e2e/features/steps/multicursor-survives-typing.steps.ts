import { expect } from '@playwright/test'
import { Then, When } from './fixtures'

// `measure-click-selects-notes.steps.ts` owns the fixture ("the measure-click
// test fixture is loaded") and the click-and-click gesture ("I click
// corner-to-corner from measure 0 to measure 2") this feature's Background
// reuses — step
// definitions are matched globally across files by playwright-bdd, so they
// don't need to be redeclared here.

async function getEditorSelectionCount(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const monacoApi = (
      window as unknown as { monaco?: typeof import('monaco-editor') }
    ).monaco
    return monacoApi?.editor.getEditors()[0]?.getSelections()?.length ?? 0
  })
}

async function getModelLines(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const monacoApi = (
      window as unknown as { monaco?: typeof import('monaco-editor') }
    ).monaco
    return (
      monacoApi?.editor.getEditors()[0]?.getModel()?.getLinesContent() ?? []
    )
  })
}

Then(
  'the Monaco editor has {int} selections',
  async ({ page }, count: number) => {
    await expect
      .poll(() => getEditorSelectionCount(page), { timeout: 3_000 })
      .toBe(count)
  },
)

Then(
  'the Monaco editor still has {int} selections',
  async ({ page }, count: number) => {
    // The regression: a keystroke used to collapse every selection but the
    // primary one back down to a single cursor (see `Editor.tsx`'s
    // snapshot/restore effect pair, which only round-trips
    // `ed.getSelection()`/`ed.setSelection()` — Monaco's singular, primary-only
    // APIs — on every `value` change).
    await expect
      .poll(() => getEditorSelectionCount(page), { timeout: 3_000 })
      .toBe(count)
  },
)

When(
  'I type {string} using the active cursors',
  async ({ page }, text: string) => {
    // Deliberately does NOT call `focusEditor()` here: that helper clicks
    // into the Monaco view-lines, which would collapse the multicursor to a
    // single caret at the click point before this step even runs. The
    // click-and-click gesture that built the multicursor already focused
    // the editor itself (see
    // `editorImperativeHandle.ts`'s `setSelections`, which ends with
    // `ed.focus()`), so typing can go straight to the keyboard.
    await page.keyboard.type(text)
  },
)

Then(
  'measures {int}, {int}, and {int} each now contain just the note {string}',
  async (
    { page },
    measure0: number,
    _measure1: number,
    _measure2: number,
    note: string,
  ) => {
    // Fixed line numbers from `measure-click-selects-notes.steps.ts`'s
    // `clickTestSource`: measure 0 is line 9, measure 1 is line 11, measure 2
    // is line 13 (blank lines in between). `measure0` etc. are asserted in
    // the Gherkin step text purely for readability; the actual indices below
    // are fixed to that fixture's known layout.
    void measure0
    const lines = await getModelLines(page)
    expect(lines[8]).toBe(`[M] ${note}`)
    expect(lines[10]).toBe(`[M] ${note}`)
    expect(lines[12]).toBe(`[M] ${note}`)
  },
)
