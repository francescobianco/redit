Feature: Status bar
  The status bar shows cursor position, help hints, and mode indicators.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: Status bar shows initial position 1:1
    Then the clone shows "00001:001" on line 25

  Scenario: Status bar updates column after typing
    When I send "hello" to both
    And I wait for the editors to settle
    Then the clone shows "00001:006" on line 25

  Scenario: Status bar updates row after Enter
    When I send "abc Enter" to both
    And I wait for the editors to settle
    Then the clone shows "00002:001" on line 25

  Scenario: Status bar shows editor name and hints
    Then the clone shows "MS-DOS Editor" on line 25
    And the clone shows "F1=Help" on line 25

  Scenario: Status bar shows menu item help in menu mode
    When I send "M-f" to both
    And I wait for the editors to settle
    Then the clone shows "F1=Help" on line 25

  Scenario: Status bar shows OVR when in overtype mode
    When I send "IC" to both
    And I wait for the editors to settle
    Then the clone shows "OVR" on line 25

  Scenario: Status bar hides OVR in insert mode
    When I send "IC IC" to both
    And I wait for the editors to settle
    Then the clone screen does not contain "OVR"
