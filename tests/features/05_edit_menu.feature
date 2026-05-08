Feature: Edit menu
  The Edit menu opens with Alt+E and shows Cut, Copy, Paste, Clear
  with their DOS keyboard shortcuts.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: Edit menu opens with Alt+E
    When I press M-e
    And I wait for the editor to settle
    Then the screen is captured

  Scenario: Edit menu shows correct items and shortcuts
    When I press M-e
    And I wait for the editor to settle
    Then the screen shows "Cut"
    And the screen shows "Shift+Del"
    And the screen shows "Copy"
    And the screen shows "Ctrl+Ins"
    And the screen shows "Paste"
    And the screen shows "Shift+Ins"
    And the screen shows "Clear"
    And the screen shows "Del"

  Scenario: Status bar shows Cut help text
    When I press M-e
    And I wait for the editor to settle
    Then the status bar shows "Deletes selected text and copies it to buffer"

  Scenario: Edit menu closes with Escape
    When I press M-e
    And I press Escape
    And I wait for the editor to settle
    Then the screen does not show "Shift+Del"
