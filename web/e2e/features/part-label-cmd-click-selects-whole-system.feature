Feature: Cmd/Ctrl-clicking a part label selects the whole system

  # Cmd/Ctrl-click(-and-click) on a part label elevates the selection from
  # "this one part's system" (a plain click / click-and-click, see
  # `part-label-range-select-system-boundary.feature`) to "every part in
  # every system the gesture touches" — see `PreviewAnchorState`'s
  # 'part-label-system' doc comment and `partLabelsInMarqueeAcrossSystems`.
  #
  # `max_measures_per_system = 1` forces each measure onto its own system, so
  # Melody's and Harmony's labels repeat twice, stacked vertically:
  #
  #   System 0 (measure 0): Melody "1 2", Harmony "5 6"
  #   System 1 (measure 1): Melody "3 4", Harmony "7 1'"

  Background:
    Given the cmd-click system fixture is loaded

  Scenario: Cmd/Ctrl-clicking one part label selects every part in that label's system
    When I Ctrl-click system 0's Melody part label
    Then 2 range-selected notes belong to part index 0, as seen in part label cmd click selects whole system
    And 2 range-selected notes belong to part index 1, as seen in part label cmd click selects whole system
    And 4 notes are range-selected in total, as seen in part label cmd click selects whole system
    And system 0's Melody label's click-target rect is marked range-active
    And system 0's Harmony label's click-target rect is marked range-active

  Scenario: Cmd/Ctrl-clicking one system's part label then another system's selects every part across both systems
    When I Ctrl-click system 0's Melody label then system 1's Melody label
    Then 4 range-selected notes belong to part index 0, as seen in part label cmd click selects whole system
    And 4 range-selected notes belong to part index 1, as seen in part label cmd click selects whole system
    And 8 notes are range-selected in total, as seen in part label cmd click selects whole system
    And system 1's Melody label's click-target rect is marked range-active
    And system 1's Harmony label's click-target rect is marked range-active
