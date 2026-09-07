Feature: Lyric range selection resolves against a part label

  Scenario: Clicking a syllable in one part then a different part's label selects every note in the swept range and just the syllable's own verse
    Given the lyric-partlabel range-selection fixture is loaded and both parts have rendered
    When I click-and-click select Melody's verse-0 syllable in measure 0 then Harmony's label in system 1
    Then 4 notes are range-selected in total, as seen in lyric partlabel range select
    And 2 syllables are range-selected in total, as seen in lyric partlabel range select
    And no note in measure 2 is range-selected, as seen in lyric partlabel range select
