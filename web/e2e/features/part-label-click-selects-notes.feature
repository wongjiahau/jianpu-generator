Feature: Clicking or click-and-click selecting part labels selects notes

  # Clicking (or click-and-click range-selecting vertically across) a part
  # label — the abbreviation drawn once per system at the label region's left
  # edge — is a shortcut for selecting every note/rest that part sounds
  # across the whole system the label sits in (see `Preview.tsx`'s
  # `getPartLabelAtPoint`/`noteCellsForPartLabels`). It reuses the same note
  # range-select highlight (`[data-note-range-selected]`) and Monaco
  # multicursor pathway (`onNoteRangeSelect`) that click-and-click sweeping a
  # marquee over individual notes uses.

  Scenario: Clicking a part label selects every note that part sounds across the system
    Given the two-part click-test fixture is loaded
    When I plain-click the Melody part label
    Then 4 notes are range-selected in total
    And 4 range-selected notes belong to part index 0
    And 0 range-selected notes belong to part index 1
    And the play button shows "Selection"
    And the Melody label's click-target rect is marked range-active
    And the Harmony label's click-target rect is not marked range-active

  Scenario: A plain click on a notes-with-lyrics part label does not also select the lyric row
    Given the notes-with-lyrics click-test fixture is loaded
    When I plain-click the Melody part label
    Then 4 notes are range-selected in total
    And 0 lyrics are range-selected in total

  Scenario: A notes-with-lyrics part label does not visually overlap its own lyric label
    Given the single-measure notes-with-lyrics fixture is loaded
    Then the part label's click-target rect does not vertically overlap the lyric label's click-target rect

  Scenario: Clicking one part label then another selects both parts notes
    Given the two-part click-test fixture is loaded
    When I click the Melody part label then click the Harmony part label
    Then 8 notes are range-selected in total
    And the Melody label's click-target rect is marked range-active
    And the Harmony label's click-target rect is marked range-active
    And the play button shows "Selection"
    When I wait 700ms for the multicursor debounce and worker round-trip
    Then 8 notes are range-selected in total
    And the Melody label's click-target rect is marked range-active
    And the Harmony label's click-target rect is marked range-active

  Scenario: The part label the gesture anchored on stays visually hovered once the pointer moves onto another label
    Given the two-part click-test fixture is loaded
    When I hover the Melody part label without clicking it
    Then the Melody label's click-target rect has a visible hover fill
    When I click the Melody label and move the pointer onto the Harmony label without clicking it
    Then the Melody label's click-target rect keeps the same hover fill while the second click is pending
