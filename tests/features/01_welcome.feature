Feature: Welcome screen
  The initial welcome dialog appears when the editor starts without a file.

  Background:
    Given both editors are started fresh

  Scenario: Initial welcome dialog shows on startup
    Then the screen text matches

  Scenario: Welcome dialog is dismissed with Enter
    When I send "Enter" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Welcome dialog is dismissed with Escape
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Status bar shows correct hint in welcome mode
    Then the clone shows "F1=Help" on line 25
    And the clone screen contains "Enter=Execute"
    And the clone screen contains "Esc=Cancel"

  Scenario: Menu bar shows all menus in welcome state
    Then the clone screen contains "File"
    And the clone screen contains "Edit"
    And the clone screen contains "Search"
    And the clone screen contains "Options"
    And the clone screen contains "Help"