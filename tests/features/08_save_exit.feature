Feature: Save and exit
  Saving a file and exiting with Ctrl+X. Unsaved changes trigger the confirm dialog.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: Ctrl+X exits immediately when file is clean
    When I send "C-x" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Ctrl+X with unsaved changes shows confirm dialog
    When I send "hello" to both
    And I send "C-x" to both
    And I wait for the editors to settle
    Then the clone screen contains "not saved"

  Scenario: Confirm exit dialog has Yes No Cancel options
    When I send "hello C-x" to both
    And I wait for the editors to settle
    Then the clone screen contains "Yes"
    And the clone screen contains "No"
    And the clone screen contains "Cancel"

  Scenario: Pressing N discards changes and exits
    When I send "hello C-x" to both
    And I wait for the editors to settle
    Then the clone screen contains "not saved"

  Scenario: Pressing Escape cancels exit and returns to editor
    When I send "hello C-x Escape" to both
    And I wait for the editors to settle
    Then the clone screen contains "hello"
    And the clone screen does not contain "not saved"

  Scenario: Ctrl+S saves without dialog when filename is known
    Given both editors are started with file "test.txt"
    When I send "hello C-s" to both
    And I wait for the editors to settle
    Then the screen text matches
