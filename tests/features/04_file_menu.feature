Feature: File menu
  The File menu opens with Alt+F and contains New, Open, Save, Save As,
  Print, and Exit. Keyboard navigation updates the status bar help text.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: File menu opens with Alt+F
    When I press M-f
    And I wait for the editor to settle
    Then the screen shows "New"
    And the screen shows "Open..."
    And the screen shows "Save"
    And the screen shows "Save As..."
    And the screen shows "Print..."
    And the screen shows "Exit"
    And the screen is captured

  Scenario: Status bar shows help for the highlighted item
    When I press M-f
    And I wait for the editor to settle
    Then the status bar shows "Removes currently loaded file from memory"

  Scenario: Down arrow selects next item and updates status bar
    When I press M-f
    And I press Down
    And I wait for the editor to settle
    Then the status bar shows "Opens a file"
    And the screen is captured

  Scenario: File menu closes with Escape
    When I press M-f
    And I press Escape
    And I wait for the editor to settle
    Then the screen does not show "Save As..."

  Scenario: F2 saves the file
    When I press F2
    And I wait for the editor to settle
    Then the screen is captured
