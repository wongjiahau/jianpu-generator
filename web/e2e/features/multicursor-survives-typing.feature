Feature: Multicursor selection survives typing

  # Regression test: click-and-click range-selecting notes across multiple
  # measures (the "measure click" fixture separates each measure's source
  # line with a blank line) pushes one Monaco selection per measure — a
  # genuine multicursor, not one big contiguous range. `Editor.tsx` used to
  # snapshot/restore only the *primary* selection
  # (`ed.getSelection()`/`ed.setSelection()`, both Monaco singular APIs) on
  # every keystroke, which silently collapsed the other cursors back down to
  # one as soon as the user started typing.

  Background:
    Given the measure-click test fixture is loaded
    When I click corner-to-corner from measure 0 to measure 2
    Then 8 notes are range-selected, as seen in measure click selects notes
    And the Monaco editor has 3 selections

  Scenario: Typing with a multi-measure multicursor keeps every cursor active
    When I type "9" using the active cursors
    Then the Monaco editor still has 3 selections
    And measures 0, 1, and 2 each now contain just the note "9"

  Scenario: A further keystroke keeps building on all 3 cursors, not just the first
    When I type "9" using the active cursors
    And I type "9" using the active cursors
    Then the Monaco editor still has 3 selections
    And measures 0, 1, and 2 each now contain just the note "99"
