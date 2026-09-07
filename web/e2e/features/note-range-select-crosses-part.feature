Feature: Note range selection crosses a part boundary

  Scenario: Clicking a note in one part then a note in another part selects every note across both parts in the swept measure range
    Given the cross-part range-selection fixture is loaded and note click targets have rendered
    When I click-and-click select the note at index 0 then the note at index 3
    Then the notes at index 0, 1, 2 and 3 are all range-selected
    And the notes at index 4, 5, 6 and 7 are not range-selected
