Feature: Note range-select highlight

  Scenario: Note range-select highlight stays visible after mouseup and the subsequent Monaco-triggered re-render
    Given the note range-select test fixture is loaded and note click targets have rendered
    And the editor is focused and jumped to line 9 to prime the measure round-trip
    When I click-and-click select across notes 0 to 2
    Then 3 notes are range-selected immediately after mouseup
    And the play-measure button switches to selection mode
    And 3 notes are still range-selected after the highlighted-documents re-render
