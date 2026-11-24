// Projected by hand from rule step (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <algorithm>
#include <cstddef>
#include <ranges>

#include "hop/geom/scalar.hpp"
#include "hop/physics/bodies.hpp"
#include "hop/physics/crawl.hpp"
#include "hop/physics/fall.hpp"
#include "hop/physics/slide.hpp"
#include "hop/physics/steer.hpp"
#include "hop/world/rules.hpp"
#include "lose.hpp"

namespace hop::world {

// The guard is the only way step fails and it runs before the first write, so the rule is atomic
// with no copy of the world and no rollback, and its lines can go to the sink as they are sent.
std::expected<void, Refusal> step(World& world, const model::Pad& pad, Lines& out) {
  using model::Mode;
  if (world.mode != Mode::Playing) {
    return std::unexpected(Refusal::Over);
  }
  const model::Level& stage = world.stage;
  const Int t = world.ticks + 1;
  world.ticks = t;
  const model::Player was = world.player;
  // Placed lazily: fall reads a mover's box only on a tick that is falling free of every solid.
  const auto tops = stage.movers | std::views::transform([t](const model::Mover& m) {
                      return physics::Placed{m.no, physics::mover_box(m, t)};
                    });

  geom::Vec3 at = was.at;
  if (was.riding) {
    for (const model::Mover& m : stage.movers) {
      if (m.no == *was.riding) {
        at = geom::plus(at, physics::mover_shift(m, t));
      }
    }
  }

  const geom::Vec3 target = physics::walk_target(pad);
  const Int accel = was.grounded ? physics::ground_accel : physics::air_accel;
  Int vy = was.vel.y;
  Int coyote = was.coyote;
  if (pad.jump && !was.jump_held && (was.grounded || was.coyote > 0)) {
    vy = physics::jump_speed;
    coyote = 0;
    out.event(t, "jump");
  } else if (!pad.jump && vy > physics::hop_cut) {
    vy = physics::hop_cut;
  }
  vy = std::max(vy - physics::gravity, -physics::max_fall);

  const physics::Axis sx = physics::slide_x(at, geom::approach(was.vel.x, target.x, accel), stage.solids);
  at.x = sx.pos;
  const physics::Axis sz = physics::slide_z(at, geom::approach(was.vel.z, target.z, accel), stage.solids);
  at.z = sz.pos;
  const physics::Fall f = physics::fall(at, vy, stage.solids, tops);
  at.y = f.y;

  if (f.grounded && !was.grounded) {
    out.event(t, "land");
  }
  world.player = model::Player{
      at,
      {sx.vel, f.vel, sz.vel},
      f.grounded,
      f.grounded ? physics::coyote_ticks : std::max(coyote - 1, 0),
      pad.jump,
      f.riding,
      physics::facing(pad, was.yaw),
  };

  // patrol keeps a crawler's number, so each entry is rewritten where it lies.
  for (model::Crawler& c : world.crawlers.values_in_place()) {
    c = physics::patrol(c);
  }

  const geom::Box body = physics::body(at);
  for (const model::Gem& g : stage.gems) {
    if (!world.taken.contains(g.no) && geom::touches(body, geom::around_centre(g.at, physics::gem_half))) {
      world.taken.insert(g.no);
      out.event(t, "gem", g.no);
    }
  }

  // `for c in crawlers.values` walks the values as they were when the loop began. The body only
  // ever removes the crawler it is looking at, so walking the live vector and staying on the same
  // index after a removal visits the same crawlers in the same order.
  bool alive = true;
  for (std::size_t i = 0; alive && i < world.crawlers.size();) {
    const model::Crawler c = world.crawlers.values()[i];
    if (!geom::touches(body, physics::crawler_box(c))) {
      ++i;
    } else if (physics::stomps(at.y, vy, c)) {
      world.crawlers.remove(c.no);
      world.player.vel.y = physics::stomp_bounce;
      world.player.grounded = false;
      world.player.riding = std::nullopt;
      out.event(t, "stomp", c.no);
    } else {
      lose(world, "hit", out);
      alive = false;
    }
  }

  if (alive && at.y < physics::kill_y) {
    lose(world, "fall", out);
    alive = false;
  }

  const geom::Box goal = geom::around_feet(stage.goal, physics::goal_half);
  if (alive && geom::touches(body, goal)) {
    world.mode = Mode::Won;
    out.event(t, "won");
  }
  return {};
}

}  // namespace hop::world
