Feature: Bar-line click-and-click selects measures

  Background:
    Given the bar-line-range-select test fixture is loaded

  Scenario: Hovering the bar line between two measures shows a range-select cursor
    When I hover the bar line between measure 0 and measure 1
    Then the bar-line click-target shows a col-resize cursor

  Scenario: Cmd/Ctrl-clicking from a bar line into a further measure selects every note in the full range
    When I Cmd/Ctrl-click-and-click from the bar line before measure 1 into measure 2's interior
    Then 4 notes are range-selected, as seen in bar line click selects measures
    And the play-measure button reads Selection

  Scenario: Plain click-and-click from a bar line into a further measure selects every note in the full range
    When I plain click-and-click from the bar line before measure 1 into measure 2's interior
    Then 4 notes are range-selected, as seen in bar line click selects measures
    And the play-measure button reads Selection
