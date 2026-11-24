# frozen_string_literal: true

# Projected by hand from app TodoRuby with @bin and @store (src/apps/ruby.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  # What the app declaration fixes: the executable's name and where state lives between runs.
  module App
    BIN = "todo"
    STORE_FILE = ".todo.json"
    STORE_ENV = "TODO_STORE"

    # The store path as configured, never resolved: errors print it this way.
    def self.store_path(env)
      configured = env[STORE_ENV]
      configured.nil? || configured.empty? ? STORE_FILE : configured
    end
  end
end
