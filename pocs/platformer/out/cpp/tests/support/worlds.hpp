// Projected by hand from the named start worlds past_the_first_ledge, running_at_the_step, under_a_low_ceiling, riding_the_mover, above_the_mover, at_the_third_gem, falling_on_the_crawler, running_at_the_crawler, about_to_fall_out, about_to_fall_out_for_the_last_time, beside_the_flag, about_to_fall_out_holding_a_gem, rising_beside_the_crawler, under_the_mover, falling_fast_onto_the_floor (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>

#include "hop/world/world.hpp"

// @given(state -> value, ...): the declared initial world with the named states replaced.
namespace worlds {

using hop::model::Crawler;
using hop::model::Player;
using hop::world::World;

// A body just past the first ledge of the meadow, still within its coyote ticks.
inline World past_the_first_ledge() {
  World w;
  w.stage = hop::levels::meadow;
  w.player = Player{.at = {1090, 0, 0}, .grounded = true, .coyote = 6};
  return w;
}

// Running at the stone step of the meadow.
inline World running_at_the_step() {
  World w;
  w.stage = hop::levels::meadow;
  w.player = Player{.at = {2400, 0, 0}, .vel = {16, 0, 0}, .grounded = true};
  return w;
}

inline constexpr std::array<hop::model::Solid, 2> box_solids{
    hop::model::Solid{{{-512, -256, -512}, {512, 0, 512}}, {0, 0, 0}},
    hop::model::Solid{{{-512, 400, -512}, {512, 600, 512}}, {0, 0, 0}},
};
inline constexpr hop::model::Level box{"box", {0, 0, 0}, box_solids, {}, {}, {}, {100000, 0, 0}};

// A one-metre room whose ceiling is lower than a jump.
inline World under_a_low_ceiling() {
  World w;
  w.stage = box;
  w.player = Player{.at = {0, 0, 0}, .grounded = true};
  return w;
}

inline World riding_the_mover() {
  World w;
  w.stage = hop::levels::ridge;
  w.player = Player{.at = {1600, 0, 0}, .grounded = true, .riding = 1};
  return w;
}

inline World above_the_mover() {
  World w;
  w.stage = hop::levels::ridge;
  w.player = Player{.at = {1600, 200, 0}};
  return w;
}

inline World at_the_third_gem() {
  World w;
  w.stage = hop::levels::meadow;
  w.player = Player{.at = {4608, 0, 0}, .grounded = true};
  return w;
}

inline World falling_on_the_crawler() {
  World w;
  w.stage = hop::levels::meadow;
  w.crawlers = {Crawler{1, {3072, 256, 0}, 2752, 3392, 1, 2}};
  w.player = Player{.at = {3072, 400, 0}, .vel = {0, -20, 0}};
  return w;
}

inline World running_at_the_crawler() {
  World w;
  w.stage = hop::levels::meadow;
  w.crawlers = {Crawler{1, {3072, 256, 0}, 2752, 3392, 1, 2}};
  w.player = Player{.at = {2900, 256, 0}, .vel = {16, 0, 0}, .grounded = true};
  return w;
}

inline World about_to_fall_out() {
  World w;
  w.stage = hop::levels::meadow;
  w.player = Player{.at = {1280, -2040, 0}, .vel = {0, -48, 0}};
  return w;
}

inline World about_to_fall_out_for_the_last_time() {
  World w = about_to_fall_out();
  w.lives = 1;
  return w;
}

inline World beside_the_flag() {
  World w;
  w.stage = hop::levels::meadow;
  w.player = Player{.at = {5000, 0, 0}, .grounded = true};
  return w;
}

// A gem already taken, then a fall.
inline World about_to_fall_out_holding_a_gem() {
  World w = about_to_fall_out();
  w.taken = {1};
  return w;
}

// Rising beside a crawler, feet above its middle.
inline World rising_beside_the_crawler() {
  World w;
  w.stage = hop::levels::meadow;
  w.crawlers = {Crawler{1, {3072, 256, 0}, 2752, 3392, 1, 2}};
  w.player = Player{.at = {2920, 330, 0}, .vel = {0, 20, 0}};
  return w;
}

// Under the mover, inside its thickness, falling.
inline World under_the_mover() {
  World w;
  w.stage = hop::levels::ridge;
  w.player = Player{.at = {1600, -200, 0}};
  return w;
}

// Falling as fast as anything falls, onto the first floor.
inline World falling_fast_onto_the_floor() {
  World w;
  w.stage = hop::levels::meadow;
  w.player = Player{.at = {0, 400, 0}, .vel = {0, -48, 0}};
  return w;
}

}  // namespace worlds
