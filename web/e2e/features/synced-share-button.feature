Feature: Synced share button

  Scenario: A viewer opening the sync link sees the current score immediately, before any owner edit
    Given clipboard permissions are granted
    And the file store is seeded with the synced score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a viewer opens the copied sync link in a new page
    Then the viewer's preview contains "Synced Score"

  Scenario: The copied sync link carries the filename as a human-readable suffix, and a viewer opening it still sees the score
    Given clipboard permissions are granted
    And the file store is seeded with the synced score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    And the copied sync link contains the filename as a human-readable suffix
    When a viewer opens the copied sync link in a new page
    Then the viewer's preview contains "Synced Score"

  Scenario: A viewer opening the sync link does not get a ?file= param populated in the URL
    Given clipboard permissions are granted
    And the file store is seeded with the synced score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a viewer opens the copied sync link in a new page
    Then the viewer's page URL has no query string

  Scenario: Sync button copies a #synced= link and shows a toast, then a dropdown offers copy/stop
    Given clipboard permissions are granted
    And local storage is cleared
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    And the copied sync link matches the synced URL hash format
    And the sync button now reads "Synced"
    When the owner clicks the sync button again
    Then the copy-sync-link and stop-sync buttons are visible
    When the owner clicks the copy-sync-link button
    Then a sync-link-copied toast is shown
    And the copied link is unchanged from before
    When the owner clicks the sync button and then the stop-sync button
    Then the stop-sync button disappears
    And the sync button reads "Sync"

  Scenario: Stopping sync marks the link ended for future loads, but a viewer already on the page isn't pushed to
    Given clipboard permissions are granted
    And the file store is seeded with the synced score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a viewer opens the copied sync link in a new page
    Then the viewer sees the preview page
    When the owner clicks the sync button and then the stop-sync button
    Then the viewer's preview contains "Synced Score"
    When the viewer reloads the page
    Then the viewer sees "This synced share has ended."
    And the viewer's preview no longer contains "Synced Score"
    When a late viewer opens the copied sync link in a new page
    Then the late viewer sees "This synced share has ended."
    And the late viewer's preview no longer contains "Synced Score"
    When the owner clicks "Sync" again
    Then a sync-link-copied toast is shown
    And the revived sync link is identical to the original link
    When the late viewer reloads the page
    Then the late viewer's preview contains "Synced Score"

  Scenario: Editing while synced does not push to the viewer until the autosave debounce fires
    Given clipboard permissions are granted
    And the file store is seeded with the synced score
    And the clock is under test control
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a viewer opens the copied sync link in a new page
    Then the viewer's preview contains "Synced Score"
    When the owner edits the synced score's title to "Edited Synced Score"
    And the viewer reloads the page
    Then the viewer's preview contains "Synced Score"
    And the viewer's preview no longer contains "Edited Synced Score"
    When the owner's autosave debounce interval elapses
    And the viewer reloads the page
    Then the viewer's preview contains "Edited Synced Score"

  Scenario: A viewer importing the synced score clears the #synced= hash and focuses the imported file
    Given clipboard permissions are granted
    And the file store is seeded with the synced score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a separate browser context opens the copied sync link as a viewer
    And the viewer clicks "Import to my scores"
    Then the viewer's shared preview banner is gone
    And the viewer's page URL has no hash
    And the viewer's file switcher shows the synced filename

  Scenario: Click-and-click selecting across measures in a synced viewer updates the play range, even without a mounted editor to round-trip the selection through
    Given clipboard permissions are granted
    And the file store is seeded with a multi-measure synced range-select score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a viewer opens the copied sync link in a new page and waits for measures to render
    Then the viewer's parts toolbar is visible and no Monaco editor is mounted
    When the viewer clicks-and-clicks from measure 0 to measure 2
    Then the viewer's play-measure button reads "Measures 1-3"
    And the viewer's measure highlight is not shown
    And the viewer's note highlight still shows after settling

  Scenario: Tapping a single note in a synced viewer only highlights that note, not its whole measure
    Given clipboard permissions are granted
    And the file store is seeded with a multi-measure synced range-select score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a viewer opens the copied sync link in a new page and waits for measures to render
    Then the viewer's parts toolbar is visible and no Monaco editor is mounted
    When the viewer taps the first note
    Then the viewer's tapped note is highlighted
    And the viewer's measure highlight is not shown

  Scenario: Tapping a bar line in a synced viewer never paints the amber measure highlight
    Given clipboard permissions are granted
    And the file store is seeded with a multi-measure synced range-select score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a viewer opens the copied sync link in a new page and waits for measures to render
    Then the viewer's parts toolbar is visible and no Monaco editor is mounted
    When the viewer taps a bar line
    Then the viewer's measure highlight is not shown

  Scenario: Clicking a section label in a synced viewer after a bar-line tap clears the bar line's stale note highlight
    Given clipboard permissions are granted
    And the file store is seeded with a two-section synced score
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When a viewer opens the copied sync link in a new page and waits for measures to render
    Then the viewer's parts toolbar is visible and no Monaco editor is mounted
    When the viewer taps a bar line
    And the viewer clicks the section label "B" in the SVG preview
    Then the viewer's note highlight is cleared

  Scenario: Re-syncing on the same file reproduces the same link, so it never needs re-sharing
    Given clipboard permissions are granted
    And local storage is cleared
    When the owner loads the app and clicks "Sync"
    Then a sync-link-copied toast is shown
    When the owner clicks the sync button and then the stop-sync button
    Then the stop-sync button disappears
    When the owner clicks "Sync" again
    Then a sync-link-copied toast is shown
    And the revived sync link is identical to the original link
