Feature: Help system
  F1 opens the Getting Started help window. The Help menu provides
  Getting Started, Keyboard reference, and About.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: F1 opens the Getting Started help window
    When I press F1
    And I wait for the editor to settle
    Then the screen shows "HELP: Getting Started"
    And the screen is captured

  Scenario: Help window closes with Escape
    When I press F1
    And I press Escape
    And I wait for the editor to settle
    Then the screen does not show "HELP: Getting Started"

  Scenario: Help menu opens with Alt+H
    When I press M-h
    And I wait for the editor to settle
    Then the screen shows "Getting Started"
    And the screen shows "Keyboard"
    And the screen shows "About..."
    And the screen is captured

  Scenario: Getting Started opens from Help menu
    When I press M-h
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "HELP: Getting Started"
    And the screen is captured

  Scenario: Keyboard help opens from Help menu
    When I press M-h
    And I press Down
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "HELP: Keyboard"
    And the screen is captured

  Scenario: About dialog opens from Help menu
    When I press M-h
    And I press Down
    And I press Down
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "redit"
    And the screen is captured
