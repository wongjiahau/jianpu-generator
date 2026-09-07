Feature: Measure click selects notes

  Scenario: Clicking a measure selects just the note under the pointer
    Given the measure-click test fixture is loaded
    When I plain-click the center of measure 1
    Then 1 note is range-selected, as seen in measure click selects notes
    And the play-measure button reads Selection, as seen in measure click selects notes

  Scenario: Cmd/Ctrl-clicking a measure selects every note in that measure
    Given the measure-click test fixture is loaded
    When I Cmd/Ctrl-click the center of measure 1
    Then 2 notes are range-selected, as seen in measure click selects notes
    And the play-measure button reads Selection, as seen in measure click selects notes

  Scenario: Cmd/Ctrl-clicking right at a measure boundary selects that measure, not its neighbor
    Given the measure-click test fixture is loaded
    When I Cmd/Ctrl-click measure 1's own left edge pixel
    Then 2 notes are range-selected, as seen in measure click selects notes
    And note ids 4 and 5 are range-selected, as seen in measure click selects notes

  Scenario: Clicking across measures selects every note in the range
    Given the measure-click test fixture is loaded
    When I click corner-to-corner from measure 0 to measure 2
    Then 8 notes are range-selected, as seen in measure click selects notes
    And the play-measure button reads Selection, as seen in measure click selects notes
    And the selection survives the debounced highlight swap with 8 notes still selected

  Scenario: Clicking a merged rest bar selects its one merged-run note cell
    Given the merged-rest test fixture is loaded
    When I plain-click the center of the merged rest bar spanning measures 1 to 3
    Then 1 note is range-selected, as seen in measure click selects notes
    And the play-measure button reads Selection, as seen in measure click selects notes
