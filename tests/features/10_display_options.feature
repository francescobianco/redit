Feature: Display options
  Options → Display opens a dialog to choose foreground color, background color,
  scroll bar visibility, and tab stop width.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: Display dialog opens from Options menu
    When I press M-o
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Display"
    And the screen is captured

  Scenario: Display dialog contains color sections
    When I press M-o
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Foreground"
    And the screen shows "Background"

  Scenario: Display dialog contains display options
    When I press M-o
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Scroll Bars"
    And the screen shows "Tab Stops"

  Scenario: Display dialog closes with Escape
    When I press M-o
    And I press Enter
    And I press Escape
    And I wait for the editor to settle
    Then the screen does not show "Foreground"
    And the screen is captured
