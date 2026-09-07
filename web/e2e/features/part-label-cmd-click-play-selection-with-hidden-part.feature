Feature: Playing a Cmd/Ctrl-click part-label selection with a hidden part

  # Regression test: Cmd/Ctrl-clicking a part label to select "every part in
  # that system" (see `part-label-cmd-click-selects-whole-system.spec.ts`) and
  # then pressing "play selection" plays the WRONG parts whenever a part
  # earlier in the declaration order is hidden.
  #
  # Root cause: `useNoteSelection.ts`'s `selectedNoteRangePlaybackInfo` resolves
  # each selected run's part name via `parts[partIndex]`, where `partIndex`
  # (`run.sourcePartIndex`) comes from `noteSpans` — fetched via the
  # `listNoteSpans` worker message *with* `enabledTracks`, so hidden parts are
  # `Vec::retain`d out and every later part's index is compacted down. `parts`
  # itself comes from the `listParts` worker message, sent with no
  # `enabledTracks` at all, so it stays the full, unfiltered, declaration-order
  # list. Looking a compacted index up in the uncompacted array resolves to the
  # wrong part whenever anything before the selected part is hidden.
  #
  # Fixture: three parts, Melody / Harmony / Bass. Harmony (the middle part) is
  # hidden, which compacts Bass from source part-index 2 down to rendered
  # part-index 1. Cmd/Ctrl-clicking Melody's label selects every visible part
  # in the system — Melody and Bass — so a correct "play selection" mutes
  # everything except Melody and Bass. The bug instead resolves the
  # compacted-index-1 run to Harmony (`parts[1]` in the unfiltered array),
  # so playback mutes Bass — a part the user actually selected — and unmutes
  # Harmony, a part that's hidden and was never selected at all.

  Scenario: Playing a Cmd/Ctrl-click part-label selection only enables the visible selected parts, even when another part is hidden
    Given the three-part hidden-part fixture is loaded with enabled-tracks capture
    When I hide the Harmony part, as seen in part label cmd click play selection with hidden part
    And I Ctrl-click the Melody part label
    Then 2 range-selected notes belong to part index 0, as seen in part label cmd click play selection with hidden part
    And 2 range-selected notes belong to part index 1, as seen in part label cmd click play selection with hidden part
    And 4 notes are range-selected in total, as seen in part label cmd click play selection with hidden part
    And the play button shows "Selection" and becomes enabled
    When I click the play button
    Then the captured enabled tracks are exactly Bass and Melody
