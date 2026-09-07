Feature: Note-lyric cross range-select

  Background:
    Given the note-lyric cross range-select test fixture is loaded and both rows have rendered

  Scenario: A click-and-click range that starts on a note and ends down across lyric syllables also selects those syllables
    When I click-and-click select from note 0's click target down and across to lyric syllable 2
    Then 3 notes are range-selected by the cross-row range
    And 3 lyric syllables are range-selected by the cross-row range

  Scenario: A click-and-click range that starts on a lyric syllable and ends up across notes also selects those notes
    When I click-and-click select from lyric syllable 0 up and across to note 2's click target
    Then 3 lyric syllables are range-selected by the cross-row range
    And 3 notes are range-selected by the cross-row range
