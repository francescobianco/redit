Feature: Search menu
  The Search menu opens with Alt+S and provides Find, Repeat Last Find, and Change.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: Search menu opens with Alt+S
    When I send "M-s" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Search menu contains exactly three items
    When I send "M-s" to both
    And I wait for the editors to settle
    Then the clone screen contains "Find..."
    And the clone screen contains "Repeat Last Find"
    And the clone screen contains "F3"
    And the clone screen contains "Change..."

  Scenario: Find dialog opens from menu
    When I send "M-s Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "Find What:"

  Scenario: Find dialog opens with Ctrl+F
    When I send "C-f" to both
    And I wait for the editors to settle
    Then the clone screen contains "Find What:"

  Scenario: Ctrl+F find and F3 repeat
    When I send "abc abc abc" to both
    And I send "C-Home C-f abc Enter" to both
    And I wait for the editors to settle
    Then the clone shows "00001:004" on line 25
    When I send "F3" to both
    And I wait for the editors to settle
    Then the clone shows "00001:008" on line 25

  Scenario: Find not found shows message
    When I send "hello" to both
    And I send "C-Home C-f zzz Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "not found"

  Scenario: Change dialog opens with Ctrl+H
    When I send "C-h" to both
    And I wait for the editors to settle
    Then the clone screen contains "Find What:"
    And the clone screen contains "Replace With:"
