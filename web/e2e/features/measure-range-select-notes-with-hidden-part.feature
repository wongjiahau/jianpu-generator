Feature: Measure click-and-click selects notes with hidden part

  Scenario: A measure click-and-click range across systems selects every visible part's notes even when another part is hidden
    Given the measure-range-select hidden-part test fixture is loaded
    When I hide the Harmony part
    Then 8 notes render across both measures
    When I Cmd/Ctrl-click-and-click from measure 0's left bar line into measure 1's interior
    Then 8 notes are range-selected, as seen in measure click selects notes with hidden part
    And Bass's measure-1 notes with ids 2 and 3 at part-index 1 are range-selected
