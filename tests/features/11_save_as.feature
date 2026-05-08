Feature: Save As dialog
  Typing text and saving with File → Save As opens a compact centered dialog
  with a file name field, a Dirs/Drives list, and OK/Cancel/Help buttons.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: Save As dialog opens from File menu
    When I type "hello world"
    And I press M-f
    And I wait for the editor to settle
    And I press Down
    And I press Down
    And I press Down
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Save As"
    And the screen is captured

  Scenario: Save As dialog shows file name field
    When I type "hello world"
    And I press M-f
    And I press Down
    And I press Down
    And I press Down
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "File Name"
    And the screen shows "Save As"
    And the screen is captured

  Scenario: Save As dialog shows Dirs/Drives section
    When I type "hello world"
    And I press M-f
    And I press Down
    And I press Down
    And I press Down
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "Dirs/Drives"
    And the screen is captured

  Scenario: Save As dialog shows OK, Cancel, Help buttons
    When I type "hello world"
    And I press M-f
    And I press Down
    And I press Down
    And I press Down
    And I press Enter
    And I wait for the editor to settle
    Then the screen shows "< OK >"
    And the screen shows "< Cancel >"
    And the screen shows "< Help >"
    And the screen is captured

  Scenario: Save As dialog closes with Escape
    When I type "hello world"
    And I press M-f
    And I press Down
    And I press Down
    And I press Down
    And I press Enter
    And I wait for the editor to settle
    And I press Escape
    And I wait for the editor to settle
    Then the screen does not show "Save As"
    And the screen is captured
