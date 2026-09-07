Feature: Pending-second-click affordance for click-and-click range selection

  # A click-and-click range gesture (see `handleAnchorClick`/`handleCommitClick`
  # in `previewClickHandler.ts`) is invisible mid-gesture without this: the
  # anchoring click's self-commit used to paint the exact same highlight color
  # a normal single-click selection uses, with no visual difference between
  # "this is the whole selection" and "this is waiting for a second click to
  # become a range". This spec covers the fix — while a gesture is anchored
  # (`usePreviewClickSelection`'s `pendingSecondClick`):
  #
  #   1. The anchor's note highlight paints in a distinct amber "pending"
  #      color instead of the normal committed-selection blue (see
  #      `[data-pending-selection]` in `index.css`).
  #   2. A "Click again to select a range" banner appears above the preview
  #      pane.
  #
  # Both revert the moment the gesture resolves — a committing second click,
  # a cancelling click on empty space, or Escape (see `cancelAnchor`) — back
  # to the normal committed-selection color, with the banner hidden.

  Background:
    Given the pending-second-click fixture is loaded

  Scenario: Anchoring a click-and-click gesture shows the pending color and banner
    When I click the first note in measure 0
    Then that note is highlighted in the pending-selection color
    And the pending-second-click banner is visible

  Scenario: Committing the second click reverts to the committed color and hides the banner
    When I click-and-click select the first note in measure 0 then the first note in measure 2
    Then the pending-second-click banner is hidden
    And the range-selected notes are highlighted in the committed-selection color

  Scenario: Cancelling the gesture on empty space hides the banner and reverts the anchor's color
    When I click the first note in measure 0
    And I click on empty space below the staff
    Then the pending-second-click banner is hidden
    And that note is highlighted in the committed-selection color

  Scenario: Pressing Escape hides the banner and reverts the anchor's color
    When I click the first note in measure 0
    And I press Escape
    Then the pending-second-click banner is hidden
    And that note is highlighted in the committed-selection color
