Feature: Edit menu
  The Edit menu opens with Alt+E and shows cut/copy/paste/clear with correct shortcuts.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: Edit menu opens with Alt+E
    When I send "M-e" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Edit menu contains expected items and shortcuts
    When I send "M-e" to both
    And I wait for the editors to settle
    Then the clone screen contains "Cut"
    And the clone screen contains "Shift+Del"
    And the clone screen contains "Copy"
    And the clone screen contains "Ctrl+Ins"
    And the clone screen contains "Paste"
    And the clone screen contains "Shift+Ins"
    And the clone screen contains "Clear"
    And the clone screen contains "Del"

  Scenario: Cut status help is shown
    When I send "M-e" to both
    And I wait for the editors to settle
    Then the clone screen contains "Deletes selected text and copies it to buffer"

  Scenario: Edit menu closes with Escape
    When I send "M-e Escape" to both
    And I wait for the editors to settle
    Then the clone screen does not contain "Shift+Del"
