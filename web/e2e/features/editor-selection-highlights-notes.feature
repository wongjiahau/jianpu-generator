Feature: Editor selection highlights notes

  Background:
    Given the editor-selection test fixture is loaded

  Scenario: Selecting a measure line in the editor highlights its notes in the preview
    When I select the whole of measure 1's note line in the editor
    Then 2 notes are range-selected, as seen in editor selection highlights notes
    And the play-measure button reads Selection, as seen in editor selection highlights notes
    When I collapse the editor selection to a caret at the start of the line
    Then 0 notes are range-selected, as seen in editor selection highlights notes
