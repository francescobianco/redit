Feature: File menu
  The File menu opens with Alt+F, shows correct items, and keyboard navigation works.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: File menu opens with Alt+F
    When I send "M-f" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: File menu contains expected items
    When I send "M-f" to both
    And I wait for the editors to settle
    Then the clone screen contains "New"
    And the clone screen contains "Open..."
    And the clone screen contains "Save"
    And the clone screen contains "Save As..."
    And the clone screen contains "Print..."
    And the clone screen contains "Exit"

  Scenario: File menu closes with Escape
    When I send "M-f Escape" to both
    And I wait for the editors to settle
    Then the clone screen does not contain "Save As..."

  Scenario: Down arrow navigates menu items
    When I send "M-f Down Down" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Status bar shows help text for highlighted item
    When I send "M-f" to both
    And I wait for the editors to settle
    Then the clone screen contains "Removes currently loaded file from memory"

  Scenario: F2 saves without opening menu
    When I send "abc F2" to both
    And I wait for the editors to settle
    Then the screen text matches
