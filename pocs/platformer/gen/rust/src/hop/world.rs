//! Module `hop.world`, printed by spec-lang from `src/hop/world.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, Opt, R, Show};
use crate::machine::Machine;
use std::collections::{BTreeMap, BTreeSet};

/// everything the screen needs, read once per frame
///
/// Record `hop.world.View`, `src/hop/world.slg:145`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct View {
    pub stage: crate::hop::model::Level,
    pub ticks: Int,
    pub player: crate::hop::model::Player,
    pub taken: BTreeSet<Int>,
    pub crawlers: Vec<crate::hop::model::Crawler>,
    pub lives: Int,
    pub mode: crate::hop::model::Mode,
}

impl Show for View {
    fn show(&self) -> String {
        rt::show::record(
            "View",
            &[
                ("stage", self.stage.show()),
                ("ticks", self.ticks.show()),
                ("player", self.player.show()),
                ("taken", self.taken.show()),
                ("crawlers", self.crawlers.show()),
                ("lives", self.lives.show()),
                ("mode", self.mode.show()),
            ],
        )
    }
}

impl View {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        stage: crate::hop::model::Level,
        ticks: Int,
        player: crate::hop::model::Player,
        taken: BTreeSet<Int>,
        crawlers: Vec<crate::hop::model::Crawler>,
        lives: Int,
        mode: crate::hop::model::Mode,
    ) -> R<Self> {
        rt::claim(crate::conform::level(&stage)?, "field `stage`")?;
        rt::claim(crate::conform::player(&player)?, "field `player`")?;
        rt::claim(crate::conform::list_crawler(&crawlers)?, "field `crawlers`")?;
        Ok(View {
            stage,
            ticks,
            player,
            taken,
            crawlers,
            lives,
            mode,
        })
    }
}

/// Fn `hop.world.mode_text`, `src/hop/world.slg:135`.
pub fn mode_text(m_: crate::hop::model::Mode) -> R<String> {
    Ok(
        match &m_ {
            crate::hop::model::Mode::Playing => String::from("playing"),
            crate::hop::model::Mode::Won => String::from("won"),
            crate::hop::model::Mode::Lost => String::from("lost"),
        },
    )
}

/// one line that sums up a run, printed after the last tick
///
/// Fn `hop.world.summary`, `src/hop/world.slg:141`.
pub fn summary(m: &mut Machine) -> R<String> {
    let tmp1 = m.world.ticks.show();
    let tmp2 = mode_text(m.world.mode.clone())?;
    let tmp3 = rt::int::count(m.world.taken.len()).show();
    let tmp4 = rt::int::count(m.world.stage.gems.len()).show();
    let tmp5 = m.world.lives.show();
    Ok(
        format!(
            "end {} {} gems {}/{} lives {} at {}",
            tmp1,
            tmp2,
            tmp3,
            tmp4,
            tmp5,
            crate::hop::geom::vec_text(m.world.player.at.clone())?,
        ),
    )
}

/// Fn `hop.world.view`, `src/hop/world.slg:148`.
pub fn view(m: &mut Machine) -> R<View> {
    let tmp1 = m.world.stage.clone();
    let tmp2 = m.world.ticks;
    let tmp3 = m.world.player.clone();
    let tmp4 = m.world.taken.clone();
    let tmp5 = m.world.crawlers.values().cloned().collect::<Vec<_>>();
    let tmp6 = m.world.lives;
    let result: View = View::new(
        tmp1,
        tmp2,
        tmp3,
        tmp4,
        tmp5,
        tmp6,
        m.world.mode.clone(),
    )?;
    rt::claim(crate::conform::view(&result)?, "the result of hop.world.view")?;
    Ok(result)
}

/// every level starts the same way: three lives, nothing taken, every crawler home
///
/// Rule `hop.world.start`, `src/hop/world.slg:26`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn start(m: &mut Machine, level: Int) -> R<()> {
    m.atomic(|m| {
        rt::claim(
            crate::conform::level_no(&level)?,
            "argument `level` of hop.world.start",
        )?;
        let l: crate::hop::model::Level = crate::hop::levels::level(level)?;
        let tmp1 = l.clone();
        rt::claim(crate::conform::level(&tmp1)?, "a write to hop.world.stage")?;
        m.world.stage = tmp1;
        m.world.ticks = 0;
        let tmp2 = crate::hop::model::Player::new(
            l.start.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
        )?;
        rt::claim(crate::conform::player(&tmp2)?, "a write to hop.world.player")?;
        m.world.player = tmp2;
        m.world.taken = BTreeSet::<Int>::new();
        let tmp3 = BTreeMap::<Int, crate::hop::model::Crawler>::new();
        rt::claim(
            crate::conform::map_int_crawler(&tmp3)?,
            "a write to hop.world.crawlers",
        )?;
        m.world.crawlers = tmp3;
        for c in l.crawlers.clone() {
            let tmp4 = c.clone();
            rt::claim(crate::conform::crawler(&tmp4)?, "a write to hop.world.crawlers")?;
            m.world.crawlers.insert(c.no, tmp4);
        }
        rt::claim(crate::conform::range_0_3(&3)?, "a write to hop.world.lives")?;
        m.world.lives = 3;
        m.world.mode = crate::hop::model::Mode::Playing;
        crate::machine::out_line(m, format!("0 level {}", l.name))?;
        Ok(())
    })
}

/// a hit or a fall costs a life and sends the hero back to the start; the last one ends the game
///
/// Rule `hop.world.lose`, `src/hop/world.slg:41`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn lose(m: &mut Machine, why: String) -> R<()> {
    m.atomic(|m| {
        if !(m.world.mode == crate::hop::model::Mode::Playing) {
            return Err(rt::fail("Over", Vec::new()));
        }
        let tmp1 = format!("{} {}", m.world.ticks.show(), why);
        crate::machine::out_line(m, tmp1)?;
        let tmp2 = rt::int::sub(m.world.lives, 1)?;
        rt::claim(crate::conform::range_0_3(&tmp2)?, "a write to hop.world.lives")?;
        m.world.lives = tmp2;
        let tmp3 = crate::hop::model::Player::new(
            m.world.stage.start.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
        )?;
        rt::claim(crate::conform::player(&tmp3)?, "a write to hop.world.player")?;
        m.world.player = tmp3;
        if m.world.lives == 0 {
            m.world.mode = crate::hop::model::Mode::Lost;
            let tmp4 = format!("{} lost", m.world.ticks.show());
            crate::machine::out_line(m, tmp4)?;
        }
        Ok(())
    })
}

/// one tick of play: carry, steer, jump, fall, then everything the hero touches
///
/// Rule `hop.world.step`, `src/hop/world.slg:53`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn step(m: &mut Machine, pad: crate::hop::model::Pad) -> R<()> {
    m.atomic(|m| {
        if !(m.world.mode == crate::hop::model::Mode::Playing) {
            return Err(rt::fail("Over", Vec::new()));
        }
        let t: Int = rt::int::add(m.world.ticks, 1)?;
        m.world.ticks = t;
        let was: crate::hop::model::Player = m.world.player.clone();
        let tops: Vec<crate::hop::physics::Placed> = rt::list::map(
            &m.world.stage.movers,
            |m_: crate::hop::model::Mover| -> R<crate::hop::physics::Placed> {
                crate::hop::physics::Placed::new(
                    m_.no,
                    crate::hop::physics::mover_box(m_.clone(), t)?,
                )
            },
        )?;
        let mut at: crate::hop::geom::Vec3 = was.at.clone();
        if matches!(&was.riding, Opt::Some(_)) {
            let ridden: Vec<crate::hop::model::Mover> = rt::list::filter(
                &m.world.stage.movers,
                |m_: crate::hop::model::Mover| -> R<bool> {
                    Ok(was.riding == Opt::Some(m_.no))
                },
            )?;
            for m_ in ridden.clone() {
                at = crate::hop::geom::plus(
                    at.clone(),
                    crate::hop::physics::mover_shift(m_.clone(), t)?,
                )?;
            }
        }
        let target: crate::hop::geom::Vec3 = crate::hop::physics::walk_target(
            pad.clone(),
        )?;
        let accel: Int = if was.grounded {
            crate::hop::physics::GROUND_ACCEL
        } else {
            crate::hop::physics::AIR_ACCEL
        };
        let mut vy: Int = was.vel.y;
        let mut coyote: Int = was.coyote;
        if (pad.jump && (!was.jump_held)) && (was.grounded || (was.coyote > 0)) {
            vy = crate::hop::physics::JUMP_SPEED;
            coyote = 0;
            crate::machine::out_line(m, format!("{} jump", t.show()))?;
        } else {
            if (!pad.jump) && (vy > crate::hop::physics::HOP_CUT) {
                vy = crate::hop::physics::HOP_CUT;
            }
        }
        vy = rt::int::sub(vy, crate::hop::physics::GRAVITY)?
            .max(rt::int::sub(0, crate::hop::physics::MAX_FALL)?);
        let tmp1 = at.clone();
        let tmp2 = crate::hop::geom::approach(was.vel.x, target.x, accel)?;
        let sx: crate::hop::physics::Axis = crate::hop::physics::slide_x(
            tmp1,
            tmp2,
            m.world.stage.solids.clone(),
        )?;
        let mut tmp3 = at.clone();
        let tmp4 = sx.pos;
        rt::claim(crate::conform::sub(&tmp4)?, "field `x`")?;
        tmp3.x = tmp4;
        at = tmp3;
        let tmp5 = at.clone();
        let tmp6 = crate::hop::geom::approach(was.vel.z, target.z, accel)?;
        let sz: crate::hop::physics::Axis = crate::hop::physics::slide_z(
            tmp5,
            tmp6,
            m.world.stage.solids.clone(),
        )?;
        let mut tmp7 = at.clone();
        let tmp8 = sz.pos;
        rt::claim(crate::conform::sub(&tmp8)?, "field `z`")?;
        tmp7.z = tmp8;
        at = tmp7;
        let tmp9 = at.clone();
        let f: crate::hop::physics::Fall = crate::hop::physics::fall(
            tmp9,
            vy,
            m.world.stage.solids.clone(),
            tops.clone(),
        )?;
        let mut tmp10 = at.clone();
        let tmp11 = f.y;
        rt::claim(crate::conform::sub(&tmp11)?, "field `y`")?;
        tmp10.y = tmp11;
        at = tmp10;
        if f.grounded && (!was.grounded) {
            crate::machine::out_line(m, format!("{} land", t.show()))?;
        }
        let tmp12 = crate::hop::model::Player::new(
            at.clone(),
            Some(crate::hop::geom::Vec3::new(sx.vel, f.vel, sz.vel)?),
            Some(f.grounded),
            Some(
                if f.grounded {
                    crate::hop::physics::COYOTE_TICKS
                } else {
                    rt::int::sub(coyote, 1)?.max(0)
                },
            ),
            Some(pad.jump),
            Some(f.riding.clone()),
            Some(crate::hop::physics::facing(pad.clone(), was.yaw)?),
        )?;
        rt::claim(crate::conform::player(&tmp12)?, "a write to hop.world.player")?;
        m.world.player = tmp12;
        for c in m.world.crawlers.values().cloned().collect::<Vec<_>>() {
            let tmp13 = crate::hop::physics::patrol(c.clone())?;
            rt::claim(
                crate::conform::crawler(&tmp13)?,
                "a write to hop.world.crawlers",
            )?;
            m.world.crawlers.insert(c.no, tmp13);
        }
        for g in m.world.stage.gems.clone() {
            if (!m.world.taken.contains(&g.no))
                && crate::hop::geom::touches(
                    crate::hop::physics::body(at.clone())?,
                    crate::hop::geom::around_centre(
                        g.at.clone(),
                        crate::hop::physics::GEM_HALF,
                    )?,
                )?
            {
                m.world.taken.insert(g.no);
                crate::machine::out_line(
                    m,
                    format!("{} gem {}", t.show(), g.no.show()),
                )?;
            }
        }
        let mut alive: bool = true;
        for c in m.world.crawlers.values().cloned().collect::<Vec<_>>() {
            if alive
                && crate::hop::geom::touches(
                    crate::hop::physics::body(at.clone())?,
                    crate::hop::physics::crawler_box(c.clone())?,
                )?
            {
                if crate::hop::physics::stomps(at.y, vy, c.clone())? {
                    let tmp14 = m.world.crawlers.clone();
                    let tmp15 = rt::list::map_remove(&tmp14, &c.no);
                    rt::claim(
                        crate::conform::map_int_crawler(&tmp15)?,
                        "a write to hop.world.crawlers",
                    )?;
                    m.world.crawlers = tmp15;
                    let tmp18 = m.world.player.clone();
                    let mut tmp16 = m.world.player.vel.clone();
                    let tmp17 = crate::hop::physics::STOMP_BOUNCE;
                    rt::claim(crate::conform::sub(&tmp17)?, "field `y`")?;
                    tmp16.y = tmp17;
                    let mut tmp19 = tmp18;
                    let tmp20 = tmp16;
                    rt::claim(crate::conform::vec3(&tmp20)?, "field `vel`")?;
                    tmp19.vel = tmp20;
                    let tmp21 = false;
                    tmp19.grounded = tmp21;
                    let tmp22 = Opt::None;
                    tmp19.riding = tmp22;
                    let tmp23 = tmp19;
                    rt::claim(
                        crate::conform::player(&tmp23)?,
                        "a write to hop.world.player",
                    )?;
                    m.world.player = tmp23;
                    crate::machine::out_line(
                        m,
                        format!("{} stomp {}", t.show(), c.no.show()),
                    )?;
                } else {
                    lose(m, String::from("hit"))?;
                    alive = false;
                }
            }
        }
        if alive && (at.y < crate::hop::physics::kill_y()?) {
            lose(m, String::from("fall"))?;
            alive = false;
        }
        let goal: crate::hop::geom::Box = crate::hop::geom::around_feet(
            m.world.stage.goal.clone(),
            crate::hop::physics::goal_half()?,
        )?;
        if alive
            && crate::hop::geom::touches(
                crate::hop::physics::body(at.clone())?,
                goal.clone(),
            )?
        {
            m.world.mode = crate::hop::model::Mode::Won;
            crate::machine::out_line(m, format!("{} won", t.show()))?;
        }
        Ok(())
    })
}

/// a body just past the first ledge of the meadow, still within its coyote ticks
///
/// Test `hop.world.past_the_first_ledge`, `src/hop/world.slg:152`.
pub fn past_the_first_ledge(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// running at the stone step of the meadow
///
/// Test `hop.world.running_at_the_step`, `src/hop/world.slg:156`.
pub fn running_at_the_step(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// a one-metre room whose ceiling is lower than a jump
///
/// Test `hop.world.under_a_low_ceiling`, `src/hop/world.slg:160`.
pub fn under_a_low_ceiling(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// Test `hop.world.riding_the_mover`, `src/hop/world.slg:173`.
pub fn riding_the_mover(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// Test `hop.world.above_the_mover`, `src/hop/world.slg:176`.
pub fn above_the_mover(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// Test `hop.world.at_the_third_gem`, `src/hop/world.slg:179`.
pub fn at_the_third_gem(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// Test `hop.world.falling_on_the_crawler`, `src/hop/world.slg:182`.
pub fn falling_on_the_crawler(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// Test `hop.world.running_at_the_crawler`, `src/hop/world.slg:189`.
pub fn running_at_the_crawler(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// Test `hop.world.about_to_fall_out`, `src/hop/world.slg:196`.
pub fn about_to_fall_out(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// Test `hop.world.about_to_fall_out_for_the_last_time`, `src/hop/world.slg:199`.
pub fn about_to_fall_out_for_the_last_time(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// Test `hop.world.beside_the_flag`, `src/hop/world.slg:202`.
pub fn beside_the_flag(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// a gem already taken, then a fall
///
/// Test `hop.world.about_to_fall_out_holding_a_gem`, `src/hop/world.slg:205`.
pub fn about_to_fall_out_holding_a_gem(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// rising beside a crawler, feet above its middle
///
/// Test `hop.world.rising_beside_the_crawler`, `src/hop/world.slg:209`.
pub fn rising_beside_the_crawler(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// under the mover, inside its thickness, falling
///
/// Test `hop.world.under_the_mover`, `src/hop/world.slg:217`.
pub fn under_the_mover(_m: &mut Machine) -> R<()> {
    Ok(())
}

/// falling as fast as anything falls, onto the first floor
///
/// Test `hop.world.falling_fast_onto_the_floor`, `src/hop/world.slg:221`.
pub fn falling_fast_onto_the_floor(_m: &mut Machine) -> R<()> {
    Ok(())
}
