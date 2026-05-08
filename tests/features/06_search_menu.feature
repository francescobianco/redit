Feature: Search menu
  Find, Repeat Last Find (F3), and Change dialogs.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed
    And I type "alpha beta alpha gamma alpha"
    And I press C-Home

  Scenario: Search menu opens with Alt+S
    When I press M-s
    And I wait for the editor to settle
    Then the screen shows "Find..."
    And the screen shows "Repeat Last Find"
    And the screen shows "F3"
    And the screen shows "Change..."
    And the screen is captured

  Scenario: Find dialog opens from Search menu
    When I press M-s
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Find What:"
    And the screen is captured

  Scenario: Find dialog opens with Ctrl+F
    When I press C-f
    And I wait for the editor to settle
    Then the screen shows "Find What:"

  Scenario: Find locates text and moves cursor
    When I press C-f
    And I type "beta"
    And I press Enter
    And I wait for the editor to settle
    Then the cursor is at line 1 column 11
    And the screen is captured

  Scenario: F3 repeats the last find
    When I press C-f
    And I type "alpha"
    And I press Enter
    And I press F3
    And I wait for the editor to settle
    Then the cursor is at line 1 column 18
    And the screen is captured

  Scenario: Find shows not-found message for missing text
    When I press C-f
    And I type "zzzzz"
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "not found"
    And the screen is captured

  Scenario: Change dialog opens with Ctrl+H
    When I press C-h
    And I wait for the editor to settle
    Then the screen shows "Find What:"
    And the screen shows "Replace With:"
    And the screen is captured
