# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: what every printed program stands on.
# The same in every generated project; change the printer, not this file.

require_relative "spec/errors"
require_relative "spec/values"
require_relative "spec/order"
require_relative "spec/show"
require_relative "spec/prims"
require_relative "spec/place"
require_relative "spec/types"
require_relative "spec/program"
require_relative "spec/machine"
