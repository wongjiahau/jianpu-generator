Feature: Note range selection resolves against a lyric label

  Scenario: Clicking a note in one part then a different part's verse label selects that part's notes and the label's own verse
    Given the note-lyriclabel range-selection fixture is loaded and both parts have rendered
    When I click-and-click select Melody's note in measure 0 then Harmony's verse label in system 1
    Then 4 notes are range-selected in total, as seen in note lyriclabel range select
    And 2 syllables are range-selected in total, as seen in note lyriclabel range select
    And no note in measure 2 is range-selected, as seen in note lyriclabel range select
