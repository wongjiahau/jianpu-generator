Feature: Play note-selection audio playback

  Scenario: Clicking the play-measure button with notes range-selected plays only the selection
    Given a single-measure four-note range-select test score is loaded with the disk cache workaround
    Then all four note click-targets are rendered in the measure
    And the play-measure button label reflects the measure under the cursor
    When I click-and-click select the first three notes in the measure
    Then the play-measure button label switches to Selection
    And the play-selection button becomes enabled once the soundfont loads
    When I click the play-selection button
    Then the play-selection button shows the playing state while still labeled Selection
    And the play-selection button eventually stops showing the playing state
