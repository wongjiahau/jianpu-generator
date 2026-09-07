Feature: Part label range selection resolves against a lyric label

  Scenario: Clicking one part's label then a different part's verse label selects every note in the swept range and just the label's own verse
    Given the partlabel-lyriclabel range-selection fixture is loaded and both parts have rendered
    When I click-and-click select Melody's label in system 0 then Harmony's verse label in system 1
    Then 4 notes are range-selected in total, as seen in partlabel lyriclabel range select
    And 2 syllables are range-selected in total, as seen in partlabel lyriclabel range select
    And no note in measure 2 is range-selected, as seen in partlabel lyriclabel range select
