Feature: Lyric label click selects syllables

  Background:
    Given the lyric label click test fixture is loaded and measure spans are primed

  Scenario: Clicking a verse label selects every syllable that verse sings across the system
    When I click the verse 0 lyric label without clicking a second time
    Then verse 0's 4 syllables are range-selected and verse 1's are not
    And the verse 0 label stays visually active but the verse 1 label does not

  Scenario: Clicking one verse label then another selects both verses syllables
    When I click the verse 0 lyric label then click the verse 1 lyric label
    Then verse 0's and verse 1's syllables are all range-selected
    And both the verse 0 and verse 1 labels stay visually active
