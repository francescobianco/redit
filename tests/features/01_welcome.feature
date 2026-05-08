Feature: Welcome screen
  When the editor starts without a file argument it shows a welcome dialog
  that can be dismissed with Enter or Escape.

  Background:
    Given the editor is open

  Scenario: Welcome dialog is visible on startup
    Then the screen shows "Welcome to the MS-DOS Editor"
    And the screen shows "Copyright (C) Microsoft Corporation"
    And the screen shows "Press Enter to see the Survival Guide"
    And the screen is captured

  Scenario: Status bar in welcome mode shows full hint line
    Then the status bar shows "F1=Help"
    And the status bar shows "Enter=Execute"
    And the status bar shows "Esc=Cancel"
    And the status bar shows "Tab=Next Field"
    And the status bar shows "Arrow=Next Item"

  Scenario: Menu bar is always visible
    Then the screen shows "File"
    And the screen shows "Edit"
    And the screen shows "Search"
    And the screen shows "Options"
    And the screen shows "Help"

  Scenario: Welcome dialog is dismissed with Enter
    When I press Enter
    And I wait for the editor to settle
    Then the screen does not show "Press Enter to see the Survival Guide"
    And the screen is captured

  Scenario: Welcome dialog is dismissed with Escape
    When I press Escape
    And I wait for the editor to settle
    Then the screen does not show "Press Enter to see the Survival Guide"
    And the screen is captured
