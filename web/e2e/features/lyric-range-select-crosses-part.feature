Feature: Lyric range selection crosses a part boundary

  Scenario: Clicking a syllable in one part then a syllable in a different part's verse selects both parts' syllables in the swept measure range
    Given the cross-part lyric range-selection fixture is loaded and both parts have rendered
    When I click-and-click select Melody's verse 0 syllable 0 then Harmony's verse 0 syllable 1
    Then Melody's and Harmony's verse 0 syllables in measures 0 through 1 are range-selected
    And no syllable in measure 2 is range-selected
    And no note is range-selected
