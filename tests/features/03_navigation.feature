Feature: Cursor navigation
  Arrow keys, Home, End, Ctrl+Home, Ctrl+End, PgUp, PgDn, and word movement.

  Background:
    Given both editors are started fresh
    When I send "Escape" to both
    And I wait for the editors to settle

  Scenario: Arrow keys move the cursor
    When I send "abc Enter def Enter ghi" to both
    And I send "Up Left Left" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: Home key moves to start of line
    When I send "hello" to both
    And I send "Home" to both
    And I wait for the editors to settle
    Then the clone shows "00001:001" on line 25

  Scenario: End key moves to end of line
    When I send "hello Home End" to both
    And I wait for the editors to settle
    Then the clone shows "00001:006" on line 25

  Scenario: Ctrl+Home moves to start of document
    When I send "abc Enter def Enter ghi" to both
    And I send "C-Home" to both
    And I wait for the editors to settle
    Then the clone shows "00001:001" on line 25

  Scenario: Ctrl+End moves to end of document
    When I send "abc Enter def Enter ghi" to both
    And I send "C-Home C-End" to both
    And I wait for the editors to settle
    Then the clone shows "00003:004" on line 25

  Scenario: Ctrl+Right moves one word right
    When I send "hello world" to both
    And I send "Home C-Right" to both
    And I wait for the editors to settle
    Then the clone shows "00001:006" on line 25

  Scenario: Ctrl+Left moves one word left
    When I send "hello world" to both
    And I send "C-Left" to both
    And I wait for the editors to settle
    Then the clone shows "00001:007" on line 25

  Scenario: PgDn scrolls down
    When I send "abc" to both
    And I send "PgDn" to both
    And I wait for the editors to settle
    Then the screen text matches

  Scenario: PgUp scrolls up
    When I send "abc" to both
    And I send "PgDn PgUp" to both
    And I wait for the editors to settle
    Then the screen text matches
