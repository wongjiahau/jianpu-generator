Feature: Note range selection crosses a page boundary

  Scenario: Clicking a note on one page then a note on the next page (after scrolling it into view) selects both endpoints
    Given the cross-page range-selection fixture is loaded and note click targets have rendered
    When I click-and-click select the first note on page 1 then the third note on page 2, scrolling the second note into view first
    Then the first note on page 1 is still range-selected
    And the third note on page 2 is range-selected
