Feature: Note range selection crosses a system boundary

  Scenario: Clicking a note in one system then a note in the next system selects every note in between
    Given the cross-system range-selection fixture is loaded and note click targets have rendered
    When I click-and-click select the note at index 0 then the note at index 4
    Then the notes at index 0, 1, 2, 3 and 4 are all range-selected
