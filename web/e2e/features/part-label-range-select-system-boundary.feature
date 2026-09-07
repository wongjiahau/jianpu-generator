Feature: Click-and-click across a part label system boundary stays scoped to that part

  # `PartLabel ↔ PartLabel` range resolution is system-agnostic (see
  # `resolve_selection_range_response` in `selection_range.rs` and
  # `part-label-range-select-crosses-system.feature`): a plain click-and-click
  # range from one system's label to a label in another system is no longer
  # clamped to the anchor's own system. But it also doesn't pick up every part
  # the pointer's vertical path happens to pass over on the way — the range is
  # derived purely from each label's own `sourcePartIndex`/measure fields, not
  # a pixel-rectangle intersection. Click-and-click selecting Melody's label
  # in one system to Melody's label in another therefore selects Melody's
  # notes across both systems, but never Harmony's, even though Harmony's
  # system-0 label sits physically between the two Melody labels on screen.
  # (Cmd/Ctrl-clicking instead unions every part in every system swept — see
  # `part-label-cmd-click-selects-whole-system.feature` — a separate, coarser
  # tool this plain click-and-click does not replace.)
  #
  # `max_measures_per_system = 1` forces each measure onto its own system, so
  # Melody's and Harmony's labels repeat twice, stacked vertically:
  #
  #   System 0 (measure 0): Melody "1 2", Harmony "5 6"
  #   System 1 (measure 1): Melody "3 4", Harmony "7 1'"

  Scenario: Click-and-click selecting a part label to the same part's label in another system selects that part across both systems, not the other part's label in between
    Given the part-label system-boundary fixture is loaded
    When I click-and-click select straight down from system 0's Melody label to system 1's Melody label
    Then 4 range-selected notes belong to part index 0, as seen in part label click system boundary
    And 0 range-selected notes belong to part index 1, as seen in part label click system boundary
    And 4 notes are range-selected in total, as seen in part label click system boundary
    And system 0's Melody label's click-target rect is marked range-active, as seen in part label click system boundary
    And system 0's Harmony label's click-target rect is not marked range-active, as seen in part label click system boundary
    And system 1's Melody label's click-target rect is marked range-active, as seen in part label click system boundary
