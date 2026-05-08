Feature: Help system
  F1 opens the Getting Started help. The Help menu provides Getting Started and Keyboard.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: F1 opens help window
    When I send "F1" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: F1 from normal mode shows Getting Started
    When I send "F1" to both
    And I wait for the editors to settle
    Then the clone screen contains "Getting Started"

  Scenario: Help window closes with Escape
    When I send "F1 Escape" to both
    And I wait for the editors to settle
    Then the clone screen does not contain "HELP: Getting Started"

  Scenario: Help menu opens with Alt+H
    When I send "M-h" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Help menu contains correct items
    When I send "M-h" to both
    And I wait for the editors to settle
    Then the clone screen contains "Getting Started"
    And the clone screen contains "Keyboard"
    And the clone screen contains "About..."

  Scenario: Getting Started help opens from menu
    When I send "M-h Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "HELP: Getting Started"

  Scenario: Keyboard help opens from menu
    When I send "M-h Down Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "HELP: Keyboard"

  Scenario: About dialog opens from Help menu
    When I send "M-h Down Down Down Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "redit"
