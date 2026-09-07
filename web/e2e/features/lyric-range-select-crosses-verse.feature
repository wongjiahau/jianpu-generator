Feature: Lyric range selection crosses a verse boundary

  Scenario: Clicking a syllable in one verse then a syllable in another verse of the same part selects both verses' syllables in the swept note-id range
    Given the cross-verse lyric range-selection fixture is loaded and both verses have rendered
    When I click-and-click select verse 0's syllable 0 then verse 1's syllable 2
    Then verses 0 and 1's syllables with note id 0 through 2 are range-selected
    And verse 2's syllables are not range-selected
    And no syllable with note id 3 is range-selected
    And no note is range-selected
