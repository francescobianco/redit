Feature: Status bar
  The status bar on row 25 always shows cursor position and context-sensitive hints.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: Initial position is line 1 column 1
    Then the status bar shows "00001:001"

  Scenario: Column advances after typing
    When I type "hello"
    Then the status bar shows "00001:006"

  Scenario: Row advances after Enter
    When I type "abc"
    And I press Enter
    Then the status bar shows "00002:001"

  Scenario: Normal mode shows editor name and F1 hint
    Then the status bar shows "MS-DOS Editor"
    And the status bar shows "F1=Help"

  Scenario: Menu mode shows item help text
    When I press M-f
    And I wait for the editor to settle
    Then the status bar shows "F1=Help"
    And the screen is captured

  Scenario: OVR indicator appears in overtype mode
    When I press IC
    Then the status bar shows "OVR"

  Scenario: OVR indicator disappears when back in insert mode
    When I press IC
    And I press IC
    Then the status bar does not show "OVR"

  Scenario: Position updates after cursor movement
    When I type "abcde"
    And I press Left
    And I press Left
    Then the status bar shows "00001:004"
    And the screen is captured
