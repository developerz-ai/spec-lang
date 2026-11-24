# frozen_string_literal: true

# Projected by hand from check_collision_boxes, key_down, read_pad and the key constants (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "screen/edge"
require_relative "model"

module Skyhop
  # The raylib edge: the only code that touches the C library.
  module Screen
    KEY_RIGHT = 262
    KEY_LEFT = 263
    KEY_DOWN_ARROW = 264
    KEY_UP = 265
    KEY_SPACE = 32
    KEY_A = 65
    KEY_D = 68
    KEY_S = 83
    KEY_W = 87

    module_function

    def check_collision_boxes(a, b) = Raylib.CheckCollisionBoxes(Edge.bounding_box(a), Edge.bounding_box(b))

    def key_down(key) = Raylib.IsKeyDown(key)

    def read_pad
      Model::Pad.new(
        left: key_down(KEY_LEFT) || key_down(KEY_A),
        right: key_down(KEY_RIGHT) || key_down(KEY_D),
        forward: key_down(KEY_UP) || key_down(KEY_W),
        back: key_down(KEY_DOWN_ARROW) || key_down(KEY_S),
        jump: key_down(KEY_SPACE)
      )
    end
  end
end

require_relative "screen/assets"
require_relative "screen/present"
