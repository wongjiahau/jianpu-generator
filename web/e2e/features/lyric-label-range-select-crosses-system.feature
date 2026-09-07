Feature: Lyric label range selection crosses a system boundary

  Scenario: Clicking a verse label in one system then the same verse's label in the next system selects every syllable that verse sings in between
    Given the cross-system lyric-label range-selection fixture is loaded and labels have rendered
    When I click-and-click select the verse 0 label in system 0 then the verse 0 label in system 1
    Then verse 0's 4 syllables across both systems are all range-selected
