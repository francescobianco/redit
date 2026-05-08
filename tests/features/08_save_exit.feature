Feature: Save and exit
  Ctrl+S saves, Ctrl+X exits. Unsaved changes trigger a confirm dialog.

  Background:
    Given the editor is open
    When the welcome dialog is dismissed

  Scenario: Ctrl+X exits immediately when document is unchanged
    When I press C-x
    And I wait for the editor to settle
    Then the screen is captured

  Scenario: Ctrl+X with unsaved changes shows a confirm dialog
    When I type "hello"
    And I press C-x
    And I wait for the editor to settle
    Then the screen shows "not saved"
    And the screen shows "Yes"
    And the screen shows "No"
    And the screen shows "Cancel"
    And the screen is captured

  Scenario: Escape cancels exit and returns to the editor
    When I type "hello"
    And I press C-x
    And I press Escape
    And I wait for the editor to settle
    Then the screen shows "hello"
    And the screen does not show "not saved"
    And the screen is captured

  Scenario: Ctrl+S with an open file saves without a dialog
    Given the editor is open with file "tests/fixtures/sample.txt"
    When I type "X"
    And I press C-s
    And I wait for the editor to settle
    Then the screen is captured
