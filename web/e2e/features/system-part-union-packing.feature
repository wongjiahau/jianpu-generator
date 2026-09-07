Feature: Union-of-parts system packing
  Systems pack purely by count (max_measures_per_system), never splitting
  early because measures have different parts or lyric verse counts. A
  system's rows are the union of every part (and, per part, every verse)
  across its measures, in [parts] declaration order. A measure missing a row
  its system has gets it padded as a full-measure rest / blank verse, with no
  note identity (no click/playback target).

  Background:
    Given parts Melody [M], Harmony [H], Bass [B] are declared in that order

  Scenario: Measures with identical parts pack exactly as before
    Given max_measures_per_system is 4
    And measures 0-3 each have notes for Melody and Harmony
    When the score is laid out
    Then Melody's part label spans measures 0 to 3 in one system
    And Harmony's part label spans measures 0 to 3 in one system

  Scenario: Differing parts no longer force an early system break
    Given max_measures_per_system is 4
    And measure 0 has notes only for Melody
    And measure 1 has notes for Melody and Harmony
    When the score is laid out
    Then Melody's part label spans measures 0 to 1 in one system
    And Harmony's part label spans measures 0 to 1 in the same system
    And measure 0 has no clickable Harmony note

  Scenario: max_measures_per_system still caps system size
    Given max_measures_per_system is 2
    And measures 0-2 each have notes for Melody and Harmony
    When the score is laid out
    Then Melody's part label appears twice, spanning measures 0-1 and 2-2

  Scenario: Part union spans three completely disjoint measures
    Given max_measures_per_system is 3
    And measure 0 has notes only for Melody
    And measure 1 has notes only for Harmony
    And measure 2 has notes for Melody and Bass
    When the score is laid out
    Then Melody's, Harmony's, and Bass's part labels each span measures 0 to 2 in one system
    And the rows are ordered top to bottom: Melody, Harmony, Bass

  Scenario: Row order follows [parts] declaration order, not first appearance
    Given max_measures_per_system is 2
    And measure 0 has notes only for Harmony
    And measure 1 has notes for Melody and Harmony
    When the score is laid out
    Then the rows are ordered top to bottom: Melody, Harmony

  Scenario: A partial trailing system unions only among its own measures
    Given max_measures_per_system is 4
    And measures 0-3 each have notes for Melody and Harmony
    And measure 4 has notes only for Bass
    When the score is laid out
    Then Melody's part label spans measures 0 to 3 in one system
    And Bass's part label spans only measure 4 in a second system
    And the first system has no Bass row

  Scenario: hide_resting_parts no longer forces an early system break
    Given max_measures_per_system is 4
    And hide_resting_parts is enabled
    And measures 0, 2, and 3 have notes for Melody and Harmony
    And measure 1 has notes only for Melody, with Harmony resting
    When the score is laid out
    Then Harmony's part label spans measures 0 to 3 in one system
    And measure 1 has no clickable Harmony note

  Scenario: A verse-count change no longer forces an early system break
    Given max_measures_per_system is 2
    And measure 0's Melody part has 1 lyric verse
    And measure 1's Melody part has 2 lyric verses
    When the score is laid out
    Then Melody's verse-0 lyric label spans measures 0 to 1 in one system
    And Melody's verse-1 lyric label spans measures 0 to 1 in the same system
    And measure 0 has no clickable verse-1 lyric

  Scenario: A merge_duplicate_measures_across_parts change still forces a new system
    Given max_measures_per_system is 4
    And measure 0 has identical Melody and Harmony notes with merge_duplicate_measures_across_parts enabled
    And measure 1 has identical Melody and Harmony notes with merge_duplicate_measures_across_parts disabled
    When the score is laid out
    Then Melody's part label appears twice, once per system

  Scenario: A part's notes merged into another part's row are not re-padded as a rest in that part's own row
    Given max_measures_per_system is 4
    And measure 0 has identical Melody and Harmony notes with merge_duplicate_measures_across_parts enabled
    And measure 1 has different notes for Melody and Harmony
    When the score is laid out
    Then Harmony's part label spans measures 0 to 1 in one system
    And measure 0 has no rest glyph in Harmony's row

  Scenario: A part's notes merged into another part's row in a later measure still show in that part's own row
    Given max_measures_per_system is 4
    And measure 0 has different notes for Melody and Harmony
    And measure 1 has identical Melody and Harmony notes with merge_duplicate_measures_across_parts enabled
    When the score is laid out
    Then Harmony's part label spans measures 0 to 1 in one system
    And measure 1 has a note glyph in Harmony's row

  Scenario: Cmd/Ctrl-clicking a part label to select the whole system highlights every note in that system
    Given max_measures_per_system is 4
    And measure 0 has different notes for Melody and Harmony
    And measure 1 has identical Melody and Harmony notes with merge_duplicate_measures_across_parts enabled
    When the score is laid out
    And I Ctrl-click Melody's part label
    Then all notes in the first system are highlighted
