Feature: Display options
  Options → Display (V1) or Colors (V2) opens a dialog to choose foreground color,
  background color, scroll bar visibility, and tab stop width.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: Display dialog opens from Options menu
    When I press M-o
    And I wait 0.5
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Display (v1) or Colors (v2)"
    And the screen is captured

  Scenario: Display dialog contains color sections
    When I press M-o
    And I wait 0.5
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Foreground"
    And the screen shows "Background"
    And the screen is captured

  Scenario: Display dialog contains display options (V1 only)
    When I press M-o
    And I wait 0.5
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Scroll Bars"
    And the screen shows "Tab Stops"
    And the screen is captured

  Scenario: Display dialog shows OK and Cancel buttons
    When I press M-o
    And I wait 0.5
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "< OK >"
    And the screen shows "< Cancel >"
    And the screen is captured

  Scenario: Display dialog closes with Escape
    When I press M-o
    And I wait 0.5
    And I press Enter
    And I press Escape
    And I wait for the editor to settle
    Then the screen does not show "Foreground"
    And the screen is captured

  Scenario: Display dialog closes with OK button
    When I press M-o
    And I wait 0.5
    And I press Enter
    And I wait for the editor to settle
    And I press Tab
    And I press Tab
    And I press Tab
    And I press Tab
    And I press Enter
    And I wait for the editor to settle
    Then the screen does not show "Foreground"
    And the screen is captured
