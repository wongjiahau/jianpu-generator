Feature: Playback cursor selection with hidden part

  Scenario: Playing a range-selected part plays its notes and moves the playback cursor onto them, even when an earlier part is hidden
    Given the playback-cursor hidden-part test fixture is loaded
    When I hide the Harmony part, as seen in playback cursor selection with hidden part
    Then 4 notes render with Harmony hidden
    When I Cmd/Ctrl-click-and-click to select Bass's two notes at the compacted part-index
    Then 2 notes are range-selected at part-index 1
    And 2 notes are range-selected in total, as seen in playback cursor selection with hidden part
    When I click the play-measure button to play the selection
    Then Bass's first selected note shows the playback cursor highlight at part-index 1
