Feature: Bar-line click selects following measure

  Background:
    Given the bar-line-click test fixture is loaded

  Scenario: Clicking a few pixels into the hit-padding of a mid-system bar line still selects the following measure
    When I Cmd/Ctrl-click a few pixels left of measure 1's left edge
    Then 2 notes are range-selected

  Scenario: Clicking a system's last bar line selects the measure before it, not the next system's first measure
    When I Cmd/Ctrl-click measure 1's right edge, which is the last bar line of its system
    Then 2 notes are range-selected
    And note ids 4 and 5 are range-selected
