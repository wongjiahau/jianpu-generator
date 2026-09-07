Feature: Bar number click selects measure

  Background:
    Given the bar-number-click test fixture is loaded

  Scenario: Hovering a measure's bar number shows a highlight background
    When I hover the bar number for measure 0
    Then the bar number rect fill is highlighted

  Scenario: Cmd/Ctrl-clicking a measure's bar number selects every note in that measure
    When I Cmd/Ctrl-click the bar number for measure 0
    Then 4 notes are range-selected, as seen in bar number click selects measure
    And the play-measure button reads Selection, as seen in bar number click selects measure

  Scenario: Plain-clicking a measure's bar number selects every note in that measure
    When I plain-click the bar number for measure 0
    Then 4 notes are range-selected, as seen in bar number click selects measure
    And the play-measure button reads Selection, as seen in bar number click selects measure
