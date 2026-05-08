Feature: Display options
  Options → Display opens the color/style dialog with foreground, background, and options.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: Options menu opens with Alt+O
    When I send "M-o" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Display dialog opens from Options menu
    When I send "M-o Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "Display"

  Scenario: Display dialog shows color sections
    When I send "M-o Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "Foreground"
    And the clone screen contains "Background"

  Scenario: Display dialog shows scroll bars option
    When I send "M-o Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "Scroll Bars"

  Scenario: Display dialog shows tab stops option
    When I send "M-o Enter" to both
    And I wait for the editors to settle
    Then the clone screen contains "Tab Stops"

  Scenario: Display dialog closes with Escape
    When I send "M-o Enter Escape" to both
    And I wait for the editors to settle
    Then the clone screen does not contain "Foreground"
