# frozen_string_literal: true

# Projected by hand from the --help line of every cli rule (docs/contract.md section 2, "Help"). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"

class HelpTest < Minitest::Test
  CONTRACT = <<~HELP
    usage: todo <command> [args]

    commands:
      add <title> [--priority low|normal|high] [--tags <tag>]...
      list [--all] [--tag <tag>]
      done <no>
      reopen <no>
      rename <no> <title>
      remove <no>
      clear
      stats
  HELP

  def test_the_text_derived_from_the_table_is_the_contracts
    assert_equal CONTRACT, Todo::CLI::Help::TEXT
  end

  def test_the_tables_are_frozen_and_shareable
    assert Ractor.shareable?(Todo::CLI::Commands::ALL)
    assert Ractor.shareable?(Todo::CLI::Help::TEXT)
  end

  def test_every_command_names_a_rule_of_the_board
    Todo::CLI::Commands::ALL.each { |command| assert_respond_to Todo::Board, command.rule }
  end
end
