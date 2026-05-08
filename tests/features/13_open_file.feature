Feature: Open file dialog
  The File Open dialog matches the original MS-DOS Editor layout.

  # DOS drive listings and host filesystem listings are environment-dependent.
  # Keep the frame, title, file-name field, buttons, and status bar comparable.
  # compare-ignore: rows=4-4 cols=74-80
  # compare-ignore: rows=7-18 cols=8-73
  # compare-ignore: rows=24-24 cols=2-79

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
