Feature: Lyric syllable independent selection

  Background:
    Given the multi-verse lyric independence fixture is loaded and both verses have rendered

  Scenario: Clicking one syllable selects only that syllable, no notes
    When I click syllable 1 of verse 0 without clicking a second time
    Then only syllable 1 of verse 0 is range-selected
    And no note is range-selected by the syllable-level interaction

  Scenario: Clicking across syllables selects exactly those cells and the matching editor text
    When I click syllable 0 then click syllable 2 of verse 0
    Then 3 lyric syllables in total are range-selected
    And no note is range-selected by the syllable-level interaction
    And the Monaco selection text is "do re mi"

  Scenario: Clicking a note directly selects just that note, no lyrics
    When I click near the top of note 1's click target without clicking a second time
    Then exactly 1 note is range-selected via the note click target
    And no lyric syllable is range-selected

  Scenario: Cmd/Ctrl-clicking a note selects the whole measure, notes and every verse of lyrics alike
    When I Ctrl-click near the top of note 1's click target
    Then exactly 4 notes are range-selected via the note click target
    And 8 lyric syllables in total are range-selected

  Scenario: Verses select independently and each syllable maps to its own verse line
    When I click syllable 1 of verse 1 without clicking a second time
    Then syllable 1 of verse 1 is range-selected but syllable 1 of verse 0 is not
    And the Monaco selection text is "dos"
