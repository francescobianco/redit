Feature: Typing text
  The editor inserts characters at the cursor position and updates the status bar.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: Typing a single character
    When I send "a" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Typing a word
    When I send "hello" to both
    And I wait for the editors to settle
    Then the clone screen contains "hello"

  Scenario: Typing moves cursor right
    When I send "abc" to both
    And I wait for the editors to settle
    Then the clone shows "00001:004" on line 25

  Scenario: Enter key creates a new line
    When I send "abc Enter def" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Backspace deletes previous character
    When I send "abc BSpace" to both
    And I wait for the editors to settle
    Then the clone screen contains "ab"
    And the clone screen does not contain "abc"

  Scenario: Delete key removes character under cursor
    When I send "abc Left DC" to both
    And I wait for the editors to settle
    Then the clone screen contains "ab"

  Scenario: Insert key toggles overtype mode
    When I send "IC" to both
    And I wait for the editors to settle
    Then the clone shows "OVR" on line 25
    When I send "IC" to both
    And I wait for the editors to settle
    Then the clone screen does not contain "OVR"