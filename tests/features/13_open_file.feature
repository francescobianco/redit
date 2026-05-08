Feature: Open file dialog
  The File Open dialog matches the original MS-DOS Editor layout.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: Open dialog opens from File menu
    When I press M-f
    And I press Down
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Open"
    And the screen shows "File Name"
    And the screen is captured
