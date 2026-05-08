Feature: Text selection, copy and paste
  Keyboard-driven selection with Shift+arrows. DOS key bindings:
  Ctrl+Ins copies, Shift+Del cuts, Shift+Ins pastes.
  Typing replaces the active selection.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: Shift+Right selects text character by character
    When I type "Hello World"
    And I press Home
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I wait for the editor to settle
    Then the screen shows "Hello World"
    And the screen is captured

  Scenario: Copy and paste selected text
    When I type "Hello World"
    And I press Home
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press C-Insert
    And I press End
    And I type " "
    And I press S-Insert
    And I wait for the editor to settle
    Then the screen shows "Hello World Hello"
    And the screen is captured

  Scenario: Shift+End selects to end of line
    When I type "Hello World"
    And I press Home
    And I press S-End
    And I wait for the editor to settle
    Then the screen shows "Hello World"
    And the screen is captured

  Scenario: Typing replaces the active selection
    When I type "Hello World"
    And I press Home
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I type "Hi"
    And I wait for the editor to settle
    Then the screen shows "Hi World"
    And the screen is captured

  Scenario: Shift+Delete cuts selected text
    When I type "Hello World"
    And I press Home
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Delete
    And I wait for the editor to settle
    Then the screen shows " World"
    And the screen is captured

  Scenario: Cut then paste restores text
    When I type "Hello World"
    And I press Home
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Right
    And I press S-Delete
    And I press End
    And I press S-Insert
    And I wait for the editor to settle
    Then the screen shows " WorldHello"
    And the screen is captured
