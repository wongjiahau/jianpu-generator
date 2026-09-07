Feature: Click-and-click from a bar number selects whole systems

  # A bar number's own click target already anchors a selection
  # unconditionally, no Cmd/Ctrl required (see
  # `bar-number-click-selects-measure.feature`). This spec covers the
  # click-and-click range gesture (no modifier needed — the same two-click
  # range gesture as `note-range-select-crosses-system.feature`): anchoring
  # on a bar number, then clicking a *second* element anywhere — a note, a
  # lyric syllable, another bar number, whatever the pointer lands on —
  # selects every part, in every system from the anchor's system through the
  # system that contains the second click's element, regardless of what that
  # element is (see the 'bar-number-system' mode in `previewAnchorState.ts`).
  #
  # This is a system-by-system selection, not a measure-by-measure one: the
  # fixture below packs 2 measures per system so the distinction is visible —
  # clicking measure 0's bar number then a note in measure 2 (system 1's
  # *first* measure) must still pick up measure 3 (the *rest* of system 1),
  # not stop at measure 2. And Harmony's notes must come along too, even
  # though neither click ever lands on Harmony's row.
  #
  # `max_measures_per_system = 2` groups measures into two-measure systems:
  #
  #   System 0 (measures 0-1): Melody "1 2 3 4", Harmony "5 6 7 1'"
  #   System 1 (measures 2-3): Melody "5 6 7 1'", Harmony "1' 7 6 5"

  Background:
    Given the bar-number click-and-click whole-systems fixture is loaded

  Scenario: Clicking a bar number twice selects every part in that bar number's system
    When I click-and-click select measure 0's bar number then measure 0's bar number
    Then 4 range-selected notes belong to part index 0, as seen in bar number click-and-click selects whole systems
    And 4 range-selected notes belong to part index 1, as seen in bar number click-and-click selects whole systems
    And 8 notes are range-selected in total, as seen in bar number click-and-click selects whole systems
    And the play-measure button reads Selection, as seen in bar number click-and-click selects whole systems

  Scenario: Clicking a bar number then a note in the next system's first measure still selects that whole system, not just the clicked measure
    When I click-and-click select measure 0's bar number then the first note in measure 2's Melody notes
    Then 8 range-selected notes belong to part index 0, as seen in bar number click-and-click selects whole systems
    And 8 range-selected notes belong to part index 1, as seen in bar number click-and-click selects whole systems
    And 16 notes are range-selected in total, as seen in bar number click-and-click selects whole systems
    And the play-measure button reads Selection, as seen in bar number click-and-click selects whole systems

  Scenario: Clicking a bar number then a lyric syllable in a later system selects every part across both systems
    When I click-and-click select measure 0's bar number then a lyric syllable in measure 2
    Then 8 range-selected notes belong to part index 0, as seen in bar number click-and-click selects whole systems
    And 8 range-selected notes belong to part index 1, as seen in bar number click-and-click selects whole systems
    And 16 notes are range-selected in total, as seen in bar number click-and-click selects whole systems
    And the play-measure button reads Selection, as seen in bar number click-and-click selects whole systems
