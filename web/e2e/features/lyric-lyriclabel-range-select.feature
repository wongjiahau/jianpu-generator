Feature: Lyric range selection resolves against a lyric label

  Scenario: Clicking a verse-0 syllable then a verse-1 label in a later system selects both verses' syllables in the swept measure range
    Given the lyric-lyriclabel range-selection fixture is loaded and both verses have rendered
    When I click-and-click select verse 0's syllable in measure 0 then the verse 1 label in system 1
    Then 4 syllables are range-selected in total, as seen in lyric lyriclabel range select
    And no syllable in measure 2 is range-selected, as seen in lyric lyriclabel range select
    And no note is range-selected, as seen in lyric lyriclabel range select
