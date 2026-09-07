Feature: Shift selected range octave from the editor toolbar

  Scenario: Octave up shifts only the selected notes
    Given the single-measure melody-bass fixture is loaded
    When I select "2 3" on the Melody line
    And I click the "Octave up" editor toolbar button
    Then the editor source contains "[Melody] 1 2' 3' 4"
    And the stored source contains "[Melody] 1 2' 3' 4"
    And the editor source still contains "[Bass] 5 6 7 1"

  Scenario: A disjoint multi-measure selection keeps the same SVG notes highlighted after an octave shift
    Given the two-measure melody fixture is loaded
    When I precisely select the disjoint notes "1" in the first measure and "6" in the second
    And I remember which notes are highlighted in the SVG preview
    And I click the "Octave up" editor toolbar button
    Then the same notes are still highlighted in the SVG preview

  Scenario: Octave down shifts only the selected notes
    Given the single-measure melody-bass fixture is loaded
    When I select "2 3" on the Melody line
    And I click the "Octave down" editor toolbar button
    Then the editor source contains "[Melody] 1 2, 3, 4"
    And the stored source contains "[Melody] 1 2, 3, 4"
    And the editor source still contains "[Bass] 5 6 7 1"

  Scenario: A selection spanning two measures shifts notes in both measures
    Given the two-measure melody fixture is loaded
    When I select from "4" to "5" across the blank line separating the measures
    And I click the "Octave up" editor toolbar button
    Then the editor source contains "[Melody] 1 2 3 4'"
    And the editor source contains "[Melody] 5' 6 7 1"
    And the stored source contains "[Melody] 1 2 3 4'"
    And the stored source contains "[Melody] 5' 6 7 1"

  Scenario: A selection spanning two measures keeps its own shape after an octave shift
    Given the two-measure melody fixture is loaded
    When I precisely select from "4" to "5" spanning the two measures
    And I click the "Octave up" editor toolbar button
    Then the editor still has exactly 1 selection range

  Scenario: A selection spanning two parts shifts notes in both parts
    Given the single-measure melody-bass fixture is loaded
    When I select from "4" on the Melody line to "5" on the Bass line
    And I click the "Octave up" editor toolbar button
    Then the editor source contains "[Melody] 1 2 3 4'"
    And the editor source contains "[Bass] 5' 6 7 1"
    And the stored source contains "[Melody] 1 2 3 4'"
    And the stored source contains "[Bass] 5' 6 7 1"

  Scenario: Clicking a part label selects its notes across every measure in the system, and octave up shifts all of them
    Given the two-measure melody-harmony click-test fixture is loaded
    When I plain-click the Melody part label
    And I click the "Octave up" editor toolbar button
    Then the editor source contains "[M] 1' 2'"
    And the editor source contains "[M] 3' 4'"
    And the editor source still contains "[H] 5 6"
    And the editor source still contains "[H] 7 1"

  Scenario: Clicking a part label keeps the same SVG notes highlighted after an octave shift
    Given the two-measure melody-harmony click-test fixture is loaded
    When I plain-click the Melody part label
    And I remember which notes are highlighted in the SVG preview
    And I click the "Octave up" editor toolbar button
    Then the same notes are still highlighted in the SVG preview

  Scenario: Clicking a part label keeps the whole-system selection after an octave shift, so a second click shifts every measure again
    Given the two-measure melody-harmony click-test fixture is loaded
    When I plain-click the Melody part label
    And I click the "Octave up" editor toolbar button
    And I click the "Octave up" editor toolbar button
    Then the editor source contains "[M] 1'' 2''"
    And the editor source contains "[M] 3'' 4''"
    And the editor source still contains "[H] 5 6"
    And the editor source still contains "[H] 7 1"

  Scenario: Shifting a tied repeat-atom note up then down restores the original note
    Given the single-measure tied-repeat-note fixture is loaded
    When I select "1~ _" on the Melody line
    And I click the "Octave up" editor toolbar button
    And I click the "Octave down" editor toolbar button
    Then the editor source contains "[Melody] 1~ _ 5 5"
    And the stored source contains "[Melody] 1~ _ 5 5"

  Scenario Outline: Octave buttons are disabled when there is no range selected
    Given the single-measure melody-bass fixture is loaded
    When I place the caret on the Melody line without selecting a range
    Then the "<button>" editor toolbar button is disabled

    Examples:
      | button      |
      | Octave up   |
      | Octave down |
