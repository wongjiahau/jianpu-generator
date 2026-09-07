Feature: Click-and-click across part labels selects lyrics too

  # Regression test: a click-and-click range across a part label is supposed
  # to select every note/rest *and* every lyric syllable that part sounds
  # across the whole system the label sits in — mirroring how 'measure' mode's
  # click-and-click resolves both `noteCellsInMeasureRange` and
  # `lyricCellsInMeasureRange` together (see
  # `measure-click-selects-lyrics.spec.ts`). A *plain click* (a single click,
  # not a click-and-click range) is deliberately narrower and selects only
  # the notes row, not the lyric row — see
  # `part-label-click-selects-notes.feature`'s
  # "plain click does not also select the lyric row" scenario.
  #
  # `usePreviewClickSelection.ts`'s `'part-label'` mode used to resolve only
  # `noteCellsForPartLabels` and never a lyric-side counterpart, so a
  # part-label range-select silently skipped every lyric row underneath the
  # swept parts. `lyricCellsForPartLabels` (in `previewLabelSelection.ts`) is
  # the fix.

  Scenario: Click-and-click vertically across part labels selects both parts' notes and the lyrics under them
    Given the part-label lyric-range-select fixture is loaded
    When I click-and-click select from the Melody part label to the Harmony part label, as seen in part label click selects lyrics
    Then 8 notes are range-selected in total, as seen in part label click selects lyrics
    And 4 lyrics are range-selected in total, as seen in part label click selects lyrics
