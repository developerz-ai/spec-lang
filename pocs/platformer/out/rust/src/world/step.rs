// Projected by hand from rule step, error Over (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt;

use super::{Out, World};
use crate::geom;
use crate::model::{Mode, Pad, Player};
use crate::physics::{self, Placed};

/// `@guard(mode == Playing else Over)`: a finished game refuses a tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Over;

impl fmt::Display for Over {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Over")
    }
}

/// At most this many movers in a level: the tops are kept on the stack. Both levels have at most one.
const MAX_MOVERS: usize = 8;

impl World {
    /// One tick of play: carry, steer, jump, fall, then everything the hero touches.
    /// The guard runs before any write, so a refusal leaves the world unchanged and sends no line.
    pub fn step(&mut self, pad: Pad, out: &mut impl Out) -> Result<(), Over> {
        if self.mode != Mode::Playing {
            return Err(Over);
        }
        let t = self.ticks + 1;
        self.ticks = t;
        let was = self.player;
        let solids = self.stage.solids;

        let mut at = was.at;
        if let Some(no) = was.riding {
            for m in self.stage.movers.iter().filter(|m| m.no == no) {
                at = geom::plus(at, physics::mover_shift(m, t));
            }
        }

        let target = physics::walk_target(pad);
        let accel = if was.grounded {
            physics::GROUND_ACCEL
        } else {
            physics::AIR_ACCEL
        };
        let mut vy = was.vel.y;
        let mut coyote = was.coyote;
        if pad.jump && !was.jump_held && (was.grounded || was.coyote > 0) {
            vy = physics::JUMP_SPEED;
            coyote = 0;
            out.line(format_args!("{t} jump"));
        } else if !pad.jump && vy > physics::HOP_CUT {
            vy = physics::HOP_CUT;
        }
        vy = (vy - physics::GRAVITY).max(-physics::MAX_FALL);

        let sx = physics::slide_x(at, geom::approach(was.vel.x, target.x, accel), solids);
        at = at.with_x(sx.pos);
        let sz = physics::slide_z(at, geom::approach(was.vel.z, target.z, accel), solids);
        at = at.with_z(sz.pos);
        let f = self.fall(at, vy, t);
        at = at.with_y(f.y);

        if f.grounded && !was.grounded {
            out.line(format_args!("{t} land"));
        }
        self.player = Player {
            at,
            vel: geom::Vec3::new(sx.vel, f.vel, sz.vel),
            grounded: f.grounded,
            coyote: if f.grounded {
                physics::COYOTE_TICKS
            } else {
                (coyote - 1).max(0)
            },
            jump_held: pad.jump,
            riding: f.riding,
            yaw: physics::facing(pad, was.yaw),
        };

        for c in self.crawlers.values_mut() {
            *c = physics::patrol(*c);
        }

        let hero = physics::body(at);
        for g in self.stage.gems {
            if !self.taken.contains(&g.no)
                && geom::touches(hero, geom::around_centre(g.at, physics::GEM_HALF))
            {
                self.taken.insert(g.no);
                out.line(format_args!("{t} gem {}", g.no));
            }
        }

        // `for c in crawlers.values` reads the map as it was when the loop began; a stomp removes only the crawler at hand,
        // so walking the keys of that snapshot in order and reading each still-present entry is the same loop.
        let mut alive = true;
        let keys: Vec<i32> = self.crawlers.keys().copied().collect();
        for no in keys {
            let Some(c) = self.crawlers.get(&no).copied() else {
                continue;
            };
            if alive && geom::touches(hero, physics::crawler_box(&c)) {
                if physics::stomps(at.y, vy, &c) {
                    self.crawlers.remove(&no);
                    self.player.vel.y = physics::STOMP_BOUNCE;
                    self.player.grounded = false;
                    self.player.riding = None;
                    out.line(format_args!("{t} stomp {no}"));
                } else {
                    self.lose("hit", out);
                    alive = false;
                }
            }
        }

        if alive && at.y < physics::KILL_Y {
            self.lose("fall", out);
            alive = false;
        }

        let goal = geom::around_feet(self.stage.goal, physics::GOAL_HALF);
        if alive && geom::touches(hero, goal) {
            self.mode = Mode::Won;
            out.line(format_args!("{t} won"));
        }
        Ok(())
    }

    /// `physics.fall` against this tick's mover tops, built on the stack.
    fn fall(&self, at: geom::Vec3, vy: i32, t: i32) -> physics::Fall {
        let movers = self.stage.movers;
        if movers.len() <= MAX_MOVERS {
            let mut tops = [Placed {
                no: 0,
                r#box: geom::Box::new(at, at),
            }; MAX_MOVERS];
            for (slot, m) in tops.iter_mut().zip(movers) {
                *slot = Placed {
                    no: m.no,
                    r#box: physics::mover_box(m, t),
                };
            }
            physics::fall(at, vy, self.stage.solids, &tops[..movers.len()])
        } else {
            let tops: Vec<Placed> = movers
                .iter()
                .map(|m| Placed {
                    no: m.no,
                    r#box: physics::mover_box(m, t),
                })
                .collect();
            physics::fall(at, vy, self.stage.solids, &tops)
        }
    }
}
