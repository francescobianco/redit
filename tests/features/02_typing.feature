Feature: Typing text
  Characters typed are inserted at the cursor position.
  The status bar cursor position updates after each keystroke.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed
    And I wait for the editor to settle

  Scenario: Typing a word inserts it in the text area
    When I type "hello"
    Then the screen shows "hello"

  Scenario: Cursor column advances with each character
    When I type "abc"
    Then the cursor is at line 1 column 4

  Scenario: Enter key moves cursor to next line
    When I type "abc"
    And I press Enter
    Then the cursor is at line 2 column 1
    And the screen is captured

  Scenario: Backspace deletes the previous character
    When I type "abcd"
    And I press BSpace
    Then the screen shows "abc"
    And the screen does not show "abcd"
    And the cursor is at line 1 column 4

  Scenario: Delete key removes character under cursor
    When I type "abcd"
    And I press Home
    And I press DC
    Then the screen shows "bcd"
    And the cursor is at line 1 column 1

  Scenario: Insert key toggles overtype indicator
    When I press IC
    Then the status bar shows "OVR"
    When I press IC
    Then the status bar does not show "OVR"

  Scenario: Typing in overtype mode replaces characters
    When I type "abcd"
    And I press Home
    And I press IC
    And I type "XY"
    Then the screen shows "XYcd"
    And the screen is captured
