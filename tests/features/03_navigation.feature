Feature: Cursor navigation
  Arrow keys, Home, End, Ctrl combinations, and Page Up/Down move the cursor.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed
    And I type "first line"
    And I press Enter
    And I type "second line"
    And I press Enter
    And I type "third line"
    And I wait for the editor to settle

  Scenario: Left arrow moves cursor back one column
    When I press Left
    Then the cursor is at line 3 column 9

  Scenario: Right arrow moves cursor forward one column
    When I press Home
    And I press Right
    Then the cursor is at line 3 column 2

  Scenario: Up arrow moves to previous line
    When I press Up
    Then the cursor is at line 2 column 10

  Scenario: Down arrow moves to next line
    When I press Up
    And I press Down
    Then the cursor is at line 3 column 10

  Scenario: Home key moves to start of line
    When I press Home
    Then the cursor is at line 3 column 1

  Scenario: End key moves to end of line
    When I press Home
    And I press End
    Then the cursor is at line 3 column 11

  Scenario: Ctrl+Home moves to start of document
    When I press C-Home
    Then the cursor is at line 1 column 1

  Scenario: Ctrl+End moves to end of document
    When I press C-Home
    And I press C-End
    Then the cursor is at line 3 column 11

  Scenario: Ctrl+Right jumps to next word
    When I press C-Home
    And I press C-Right
    Then the cursor is at line 1 column 6

  Scenario: Ctrl+Left jumps to previous word
    When I press C-Left
    Then the cursor is at line 3 column 7

  Scenario: Page Down scrolls the view
    When I press C-Home
    And I press PgDn
    And I wait for the editor to settle
    Then the screen is captured

  Scenario: Page Up scrolls back up
    When I press C-Home
    And I press PgDn
    And I press PgUp
    And I wait for the editor to settle
    Then the screen is captured
