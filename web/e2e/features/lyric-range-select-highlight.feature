Feature: Lyric click-and-click range-select highlight

  Background:
    Given the lyric range-select test fixture is loaded and the first measure has rendered

  Scenario: Click-and-click across lyric syllables selects the syllables, not their underlying notes
    When I click-and-click select from lyric syllable 0 to lyric syllable 2
    Then lyric syllables 0, 1 and 2 are range-selected
    And no note is range-selected

  Scenario: Clicking a single lyric syllable selects only that syllable, not the note
    When I click lyric syllable 1 once
    Then only lyric syllable 1 is range-selected
    And no note is range-selected
