Feature: Note range selection resolves against a part label

  Scenario: Clicking a note in one part then a different part's label selects every note in the swept part/measure range
    Given the note-partlabel range-selection fixture is loaded and both parts have rendered
    When I click-and-click select Melody's note in measure 0 then Harmony's label in system 1
    Then 4 notes are range-selected in total, as seen in note partlabel range select
    And no note in measure 2 is range-selected, as seen in note partlabel range select
