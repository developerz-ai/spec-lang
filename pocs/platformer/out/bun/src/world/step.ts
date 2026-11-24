// Projected by hand from rule step, rule lose (joined into step) (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { aroundCentre, aroundFeet, touches } from "../geom/box.ts";
import { approach } from "../geom/scalar.ts";
import { plus, type Vec3 } from "../geom/vec3.ts";
import type { Crawler } from "../model/level.ts";
import type { Pad } from "../model/pad.ts";
import { type Mode, player as newPlayer, type Player } from "../model/player.ts";
import { body, crawlerBox, moverBox, moverShift } from "../physics/bodies.ts";
import * as k from "../physics/constants.ts";
import { fall } from "../physics/fall.ts";
import { patrol, stomps } from "../physics/patrol.ts";
import { slideX, slideZ } from "../physics/slide.ts";
import { facing, walkTarget } from "../physics/walk.ts";
import type { Outcome } from "./outcome.ts";
import type { World } from "./world.ts";

/** One tick of play: carry, steer, jump, fall, then everything the hero touches. */
export function step(w: World, pad: Pad): Outcome {
  if (w.mode !== "Playing") return { ok: false, refusal: "Over" };
  const stage = w.stage;
  const lines: string[] = [];
  const t = w.ticks + 1;
  const was = w.player;
  const tops = stage.movers.map((m) => ({ no: m.no, box: moverBox(m, t) }));

  let at: Vec3 = was.at;
  if (was.riding !== null) {
    for (const m of stage.movers) if (m.no === was.riding) at = plus(at, moverShift(m, t));
  }

  const target = walkTarget(pad);
  const accel = was.grounded ? k.groundAccel : k.airAccel;
  let vy = was.vel.y;
  let coyote = was.coyote;
  if (pad.jump && !was.jumpHeld && (was.grounded || was.coyote > 0)) {
    vy = k.jumpSpeed;
    coyote = 0;
    lines.push(`${t} jump`);
  } else if (!pad.jump && vy > k.hopCut) {
    vy = k.hopCut;
  }
  vy = Math.max(vy - k.gravity, -k.maxFall);

  const sx = slideX(at, approach(was.vel.x, target.x, accel), stage.solids);
  at = { x: sx.pos, y: at.y, z: at.z };
  const sz = slideZ(at, approach(was.vel.z, target.z, accel), stage.solids);
  at = { x: at.x, y: at.y, z: sz.pos };
  const f = fall(at, vy, stage.solids, tops);
  at = { x: at.x, y: f.y, z: at.z };

  if (f.grounded && !was.grounded) lines.push(`${t} land`);
  let player: Player = {
    at,
    vel: { x: sx.vel, y: f.vel, z: sz.vel },
    grounded: f.grounded,
    coyote: f.grounded ? k.coyoteTicks : Math.max(coyote - 1, 0),
    jumpHeld: pad.jump,
    riding: f.riding,
    yaw: facing(pad, was.yaw),
  };

  const patrolled = w.crawlers.map(patrol);
  let crawlers: readonly Crawler[] = patrolled;

  const here = body(at);
  let taken = w.taken;
  for (const g of stage.gems) {
    if (!taken.has(g.no) && touches(here, aroundCentre(g.at, k.gemHalf))) {
      taken = new Set(taken).add(g.no);
      lines.push(`${t} gem ${g.no}`);
    }
  }

  let lives = w.lives;
  let mode: Mode = "Playing";
  /** `lose(why)`: its guard, `mode == Playing`, holds wherever step calls it. */
  const lose = (why: string): void => {
    lines.push(`${t} ${why}`);
    lives -= 1;
    player = newPlayer(stage.start);
    if (lives === 0) {
      mode = "Lost";
      lines.push(`${t} lost`);
    }
  };

  let alive = true;
  for (const c of patrolled) {
    if (alive && touches(here, crawlerBox(c))) {
      if (stomps(at.y, vy, c)) {
        crawlers = crawlers.filter((other) => other.no !== c.no);
        player = { ...player, vel: { ...player.vel, y: k.stompBounce }, grounded: false, riding: null };
        lines.push(`${t} stomp ${c.no}`);
      } else {
        lose("hit");
        alive = false;
      }
    }
  }

  if (alive && at.y < k.killY) {
    lose("fall");
    alive = false;
  }

  if (alive && touches(here, aroundFeet(stage.goal, k.goalHalf))) {
    mode = "Won";
    lines.push(`${t} won`);
  }

  return { ok: true, world: { stage, ticks: t, player, taken, crawlers, lives, mode }, lines };
}
