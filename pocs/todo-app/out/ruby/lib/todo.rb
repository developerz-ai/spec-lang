# frozen_string_literal: true

# Projected by hand from the modules todo.model, todo.render, todo.board and app TodoRuby (src/todo/*.slg, src/apps/ruby.slg). Stand-in for spec-lang build output; change the spec, not this file.

# The whole program, for library use and tests. The executable loads only what one command needs.
require_relative "todo/app"
require_relative "todo/model"
require_relative "todo/render"
require_relative "todo/board"
require_relative "todo/store"
require_relative "todo/cli"
require_relative "todo/cli/dispatch"
