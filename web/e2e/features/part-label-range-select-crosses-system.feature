Feature: Part label range selection crosses a system boundary

  Scenario: Clicking a part label in one system then the same part's label in the next system selects every note that part sounds in between
    Given the cross-system part-label range-selection fixture is loaded and labels have rendered
    When I click-and-click select the Melody label in system 0 then the Melody label in system 1
    Then 4 notes are range-selected in total, as seen in part label range select crosses system
    And 4 range-selected notes belong to part index 0, as seen in part label range select crosses system
