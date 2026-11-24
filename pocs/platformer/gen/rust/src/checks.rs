//! Every check of the program, as `spec-lang test` names and runs it.

use crate::rt::{self, Int, Opt, R};
use crate::rt::rng::Rng;
use crate::rt::runner::{Body, Check, Given, LawBody, LawDecl, TestDecl, Trace, What};
use crate::machine::{Machine, Pools};
use std::collections::BTreeSet;

/// `@given(hop.world.stage -> ..)` of `hop.world.past_the_first_ledge`.
fn given_25_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.past_the_first_ledge`.
fn given_25_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(1090, 0, 0)?,
        None,
        Some(true),
        Some(6),
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.running_at_the_step`.
fn given_26_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.running_at_the_step`.
fn given_26_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(2400, 0, 0)?,
        Some(crate::hop::geom::Vec3::new(16, 0, 0)?),
        Some(true),
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.under_a_low_ceiling`.
fn given_27_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Level::new(
        String::from("box"),
        crate::hop::geom::Vec3::new(0, 0, 0)?,
        vec![
            crate::hop::model::Solid::new(
                crate::hop::geom::Box::new(
                    crate::hop::geom::Vec3::new(
                        rt::int::neg(512)?,
                        rt::int::neg(256)?,
                        rt::int::neg(512)?,
                    )?,
                    crate::hop::geom::Vec3::new(512, 0, 512)?,
                )?,
                crate::hop::model::Rgb::new(0, 0, 0)?,
            )?,
            crate::hop::model::Solid::new(
                crate::hop::geom::Box::new(
                    crate::hop::geom::Vec3::new(
                        rt::int::neg(512)?,
                        400,
                        rt::int::neg(512)?,
                    )?,
                    crate::hop::geom::Vec3::new(512, 600, 512)?,
                )?,
                crate::hop::model::Rgb::new(0, 0, 0)?,
            )?,
        ],
        None,
        None,
        None,
        crate::hop::geom::Vec3::new(100000, 0, 0)?,
    )?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.under_a_low_ceiling`.
fn given_27_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(0, 0, 0)?,
        None,
        Some(true),
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.riding_the_mover`.
fn given_28_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::ridge()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.riding_the_mover`.
fn given_28_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(1600, 0, 0)?,
        None,
        Some(true),
        None,
        None,
        Some(Opt::Some(1)),
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.above_the_mover`.
fn given_29_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::ridge()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.above_the_mover`.
fn given_29_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(1600, 200, 0)?,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.at_the_third_gem`.
fn given_30_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.at_the_third_gem`.
fn given_30_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(4608, 0, 0)?,
        None,
        Some(true),
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.falling_on_the_crawler`.
fn given_31_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.crawlers -> ..)` of `hop.world.falling_on_the_crawler`.
fn given_31_1(m: &mut Machine) -> R<()> {
    let v = rt::map_of(
        vec![
            (
                1,
                crate::hop::model::Crawler::new(
                    1,
                    crate::hop::geom::Vec3::new(3072, 256, 0)?,
                    2752,
                    3392,
                    1,
                    2,
                )?,
            ),
        ],
    );
    rt::claim(
        crate::conform::map_int_crawler(&v)?,
        "the `@given` value of hop.world.crawlers",
    )?;
    m.world.crawlers = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.falling_on_the_crawler`.
fn given_31_2(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(3072, 400, 0)?,
        Some(crate::hop::geom::Vec3::new(0, rt::int::neg(20)?, 0)?),
        None,
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.running_at_the_crawler`.
fn given_32_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.crawlers -> ..)` of `hop.world.running_at_the_crawler`.
fn given_32_1(m: &mut Machine) -> R<()> {
    let v = rt::map_of(
        vec![
            (
                1,
                crate::hop::model::Crawler::new(
                    1,
                    crate::hop::geom::Vec3::new(3072, 256, 0)?,
                    2752,
                    3392,
                    1,
                    2,
                )?,
            ),
        ],
    );
    rt::claim(
        crate::conform::map_int_crawler(&v)?,
        "the `@given` value of hop.world.crawlers",
    )?;
    m.world.crawlers = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.running_at_the_crawler`.
fn given_32_2(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(2900, 256, 0)?,
        Some(crate::hop::geom::Vec3::new(16, 0, 0)?),
        Some(true),
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.about_to_fall_out`.
fn given_33_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.about_to_fall_out`.
fn given_33_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(1280, rt::int::neg(2040)?, 0)?,
        Some(crate::hop::geom::Vec3::new(0, rt::int::neg(48)?, 0)?),
        None,
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.about_to_fall_out_for_the_last_time`.
fn given_34_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.lives -> ..)` of `hop.world.about_to_fall_out_for_the_last_time`.
fn given_34_1(m: &mut Machine) -> R<()> {
    let v = 1;
    rt::claim(crate::conform::range_0_3(&v)?, "the `@given` value of hop.world.lives")?;
    m.world.lives = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.about_to_fall_out_for_the_last_time`.
fn given_34_2(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(1280, rt::int::neg(2040)?, 0)?,
        Some(crate::hop::geom::Vec3::new(0, rt::int::neg(48)?, 0)?),
        None,
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.beside_the_flag`.
fn given_35_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.beside_the_flag`.
fn given_35_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(5000, 0, 0)?,
        None,
        Some(true),
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.about_to_fall_out_holding_a_gem`.
fn given_36_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.taken -> ..)` of `hop.world.about_to_fall_out_holding_a_gem`.
fn given_36_1(m: &mut Machine) -> R<()> {
    let v = BTreeSet::from([1]);
    m.world.taken = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.about_to_fall_out_holding_a_gem`.
fn given_36_2(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(1280, rt::int::neg(2040)?, 0)?,
        Some(crate::hop::geom::Vec3::new(0, rt::int::neg(48)?, 0)?),
        None,
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.rising_beside_the_crawler`.
fn given_37_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.crawlers -> ..)` of `hop.world.rising_beside_the_crawler`.
fn given_37_1(m: &mut Machine) -> R<()> {
    let v = rt::map_of(
        vec![
            (
                1,
                crate::hop::model::Crawler::new(
                    1,
                    crate::hop::geom::Vec3::new(3072, 256, 0)?,
                    2752,
                    3392,
                    1,
                    2,
                )?,
            ),
        ],
    );
    rt::claim(
        crate::conform::map_int_crawler(&v)?,
        "the `@given` value of hop.world.crawlers",
    )?;
    m.world.crawlers = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.rising_beside_the_crawler`.
fn given_37_2(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(2920, 330, 0)?,
        Some(crate::hop::geom::Vec3::new(0, 20, 0)?),
        None,
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.under_the_mover`.
fn given_38_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::ridge()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.under_the_mover`.
fn given_38_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(1600, rt::int::neg(200)?, 0)?,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// `@given(hop.world.stage -> ..)` of `hop.world.falling_fast_onto_the_floor`.
fn given_39_0(m: &mut Machine) -> R<()> {
    let v = crate::hop::levels::meadow()?;
    rt::claim(crate::conform::level(&v)?, "the `@given` value of hop.world.stage")?;
    m.world.stage = v;
    Ok(())
}

/// `@given(hop.world.player -> ..)` of `hop.world.falling_fast_onto_the_floor`.
fn given_39_1(m: &mut Machine) -> R<()> {
    let v = crate::hop::model::Player::new(
        crate::hop::geom::Vec3::new(0, 400, 0)?,
        Some(crate::hop::geom::Vec3::new(0, rt::int::neg(48)?, 0)?),
        None,
        None,
        None,
        None,
        None,
    )?;
    rt::claim(crate::conform::player(&v)?, "the `@given` value of hop.world.player")?;
    m.world.player = v;
    Ok(())
}

/// A test with parameters has no one end world.
fn no_end(_: &mut Machine) -> R<()> {
    Err(rt::stop("E-TYPE-MISMATCH", "a test with parameters has no one end world"))
}

/// The parameters of `hop.laws.never_inside_a_solid` besides its trace, drawn after it.
fn law_0(_m: &mut Machine, _rng: &mut Rng, _pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::hop::laws::never_inside_a_solid(
            m,
            t,
        )),
    )
}

/// The parameters of `hop.laws.speed_is_bounded` besides its trace, drawn after it.
fn law_1(_m: &mut Machine, _rng: &mut Rng, _pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::hop::laws::speed_is_bounded(
            m,
            t,
        )),
    )
}

/// The parameters of `hop.laws.taken_only_grows` besides its trace, drawn after it.
fn law_2(_m: &mut Machine, _rng: &mut Rng, _pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::hop::laws::taken_only_grows(
            m,
            t,
        )),
    )
}

/// The parameters of `hop.laws.lives_never_rise` besides its trace, drawn after it.
fn law_3(_m: &mut Machine, _rng: &mut Rng, _pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::hop::laws::lives_never_rise(
            m,
            t,
        )),
    )
}

/// The parameters of `hop.laws.lost_means_no_lives` besides its trace, drawn after it.
fn law_4(_m: &mut Machine, _rng: &mut Rng, _pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::hop::laws::lost_means_no_lives(
            m,
            t,
        )),
    )
}

/// The parameters of `hop.laws.over_is_final` besides its trace, drawn after it.
fn law_5(_m: &mut Machine, _rng: &mut Rng, _pools: &Pools) -> R<LawBody> {
    Ok(Box::new(move |m: &mut Machine, t: Trace| crate::hop::laws::over_is_final(m, t)))
}

/// The parameters of `hop.laws.one_tick_per_step` besides its trace, drawn after it.
fn law_6(_m: &mut Machine, _rng: &mut Rng, _pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::hop::laws::one_tick_per_step(
            m,
            t,
        )),
    )
}

/// The parameters of `hop.laws.taken_are_real_gems` besides its trace, drawn after it.
fn law_7(_m: &mut Machine, _rng: &mut Rng, _pools: &Pools) -> R<LawBody> {
    Ok(
        Box::new(move |m: &mut Machine, t: Trace| crate::hop::laws::taken_are_real_gems(
            m,
            t,
        )),
    )
}

/// `hop.geom.plus case[0]`.
fn case_0(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::geom::plus(
            crate::hop::geom::Vec3::new(1, 2, 3)?,
            crate::hop::geom::Vec3::new(10, 20, 30)?,
        ),
    )?;
    let want: crate::hop::geom::Vec3 = crate::hop::geom::Vec3::new(11, 22, 33)?;
    Ok(got == Ok(want))
}

/// `hop.geom.around_feet case[0]`.
fn case_1(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::geom::around_feet(
            crate::hop::geom::Vec3::new(0, 0, 0)?,
            crate::hop::geom::Vec3::new(64, 192, 64)?,
        ),
    )?;
    let want: crate::hop::geom::Box = crate::hop::geom::Box::new(
        crate::hop::geom::Vec3::new(rt::int::neg(64)?, 0, rt::int::neg(64)?)?,
        crate::hop::geom::Vec3::new(64, 192, 64)?,
    )?;
    Ok(got == Ok(want))
}

/// `hop.geom.overlaps case[0]`.
fn case_2(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::geom::overlaps(
            crate::hop::geom::Box::new(
                crate::hop::geom::Vec3::new(0, 0, 0)?,
                crate::hop::geom::Vec3::new(2, 2, 2)?,
            )?,
            crate::hop::geom::Box::new(
                crate::hop::geom::Vec3::new(1, 1, 1)?,
                crate::hop::geom::Vec3::new(3, 3, 3)?,
            )?,
        ),
    )?;
    let want: bool = true;
    Ok(got == Ok(want))
}

/// `hop.geom.overlaps case[1]`.
fn case_3(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::geom::overlaps(
            crate::hop::geom::Box::new(
                crate::hop::geom::Vec3::new(0, 0, 0)?,
                crate::hop::geom::Vec3::new(2, 2, 2)?,
            )?,
            crate::hop::geom::Box::new(
                crate::hop::geom::Vec3::new(2, 0, 0)?,
                crate::hop::geom::Vec3::new(4, 2, 2)?,
            )?,
        ),
    )?;
    let want: bool = false;
    Ok(got == Ok(want))
}

/// `hop.geom.touches case[0]`.
fn case_4(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::geom::touches(
            crate::hop::geom::Box::new(
                crate::hop::geom::Vec3::new(0, 0, 0)?,
                crate::hop::geom::Vec3::new(2, 2, 2)?,
            )?,
            crate::hop::geom::Box::new(
                crate::hop::geom::Vec3::new(2, 0, 0)?,
                crate::hop::geom::Vec3::new(4, 2, 2)?,
            )?,
        ),
    )?;
    let want: bool = true;
    Ok(got == Ok(want))
}

/// `hop.geom.touches case[1]`.
fn case_5(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::geom::touches(
            crate::hop::geom::Box::new(
                crate::hop::geom::Vec3::new(0, 0, 0)?,
                crate::hop::geom::Vec3::new(2, 2, 2)?,
            )?,
            crate::hop::geom::Box::new(
                crate::hop::geom::Vec3::new(3, 0, 0)?,
                crate::hop::geom::Vec3::new(4, 2, 2)?,
            )?,
        ),
    )?;
    let want: bool = false;
    Ok(got == Ok(want))
}

/// `hop.geom.triangle case[0]`.
fn case_6(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::triangle(0, 4))?;
    let want: Int = 0;
    Ok(got == Ok(want))
}

/// `hop.geom.triangle case[1]`.
fn case_7(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::triangle(3, 4))?;
    let want: Int = 3;
    Ok(got == Ok(want))
}

/// `hop.geom.triangle case[2]`.
fn case_8(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::triangle(4, 4))?;
    let want: Int = 4;
    Ok(got == Ok(want))
}

/// `hop.geom.triangle case[3]`.
fn case_9(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::triangle(5, 4))?;
    let want: Int = 3;
    Ok(got == Ok(want))
}

/// `hop.geom.triangle case[4]`.
fn case_10(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::triangle(8, 4))?;
    let want: Int = 0;
    Ok(got == Ok(want))
}

/// `hop.geom.triangle case[5]`.
fn case_11(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::triangle(9, 4))?;
    let want: Int = 1;
    Ok(got == Ok(want))
}

/// `hop.geom.approach case[0]`.
fn case_12(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::approach(0, 3, 2))?;
    let want: Int = 2;
    Ok(got == Ok(want))
}

/// `hop.geom.approach case[1]`.
fn case_13(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::approach(5, 3, 2))?;
    let want: Int = 3;
    Ok(got == Ok(want))
}

/// `hop.geom.approach case[2]`.
fn case_14(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::approach(5, 0, 2))?;
    let want: Int = 3;
    Ok(got == Ok(want))
}

/// `hop.geom.approach case[3]`.
fn case_15(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::geom::approach(rt::int::neg(5)?, 0, 1))?;
    let want: Int = rt::int::neg(4)?;
    Ok(got == Ok(want))
}

/// `hop.geom.vec_text case[0]`.
fn case_16(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::geom::vec_text(crate::hop::geom::Vec3::new(1, rt::int::neg(2)?, 3)?),
    )?;
    let want: String = String::from("1,-2,3");
    Ok(got == Ok(want))
}

/// `hop.levels.q case[0]`.
fn case_25(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::levels::q(4))?;
    let want: Int = 256;
    Ok(got == Ok(want))
}

/// `hop.levels.q case[1]`.
fn case_26(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::levels::q(rt::int::neg(2)?))?;
    let want: Int = rt::int::neg(128)?;
    Ok(got == Ok(want))
}

/// `hop.physics.walk_target case[0]`.
fn case_27(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::walk_target(
            crate::hop::model::Pad::new(None, None, None, None, None)?,
        ),
    )?;
    let want: crate::hop::geom::Vec3 = crate::hop::geom::Vec3::new(0, 0, 0)?;
    Ok(got == Ok(want))
}

/// `hop.physics.walk_target case[1]`.
fn case_28(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::walk_target(
            crate::hop::model::Pad::new(None, Some(true), None, None, None)?,
        ),
    )?;
    let want: crate::hop::geom::Vec3 = crate::hop::geom::Vec3::new(16, 0, 0)?;
    Ok(got == Ok(want))
}

/// `hop.physics.walk_target case[2]`.
fn case_29(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::walk_target(
            crate::hop::model::Pad::new(Some(true), None, Some(true), None, None)?,
        ),
    )?;
    let want: crate::hop::geom::Vec3 = crate::hop::geom::Vec3::new(
        rt::int::neg(11)?,
        0,
        rt::int::neg(11)?,
    )?;
    Ok(got == Ok(want))
}

/// `hop.physics.walk_target case[3]`.
fn case_30(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::walk_target(
            crate::hop::model::Pad::new(Some(true), Some(true), None, None, None)?,
        ),
    )?;
    let want: crate::hop::geom::Vec3 = crate::hop::geom::Vec3::new(0, 0, 0)?;
    Ok(got == Ok(want))
}

/// `hop.physics.facing case[0]`.
fn case_31(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::facing(
            crate::hop::model::Pad::new(None, None, None, None, None)?,
            90,
        ),
    )?;
    let want: Int = 90;
    Ok(got == Ok(want))
}

/// `hop.physics.facing case[1]`.
fn case_32(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::facing(
            crate::hop::model::Pad::new(None, Some(true), None, None, None)?,
            90,
        ),
    )?;
    let want: Int = 0;
    Ok(got == Ok(want))
}

/// `hop.physics.facing case[2]`.
fn case_33(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::facing(
            crate::hop::model::Pad::new(None, None, Some(true), None, None)?,
            0,
        ),
    )?;
    let want: Int = 90;
    Ok(got == Ok(want))
}

/// `hop.physics.facing case[3]`.
fn case_34(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::facing(
            crate::hop::model::Pad::new(Some(true), None, None, Some(true), None)?,
            0,
        ),
    )?;
    let want: Int = 225;
    Ok(got == Ok(want))
}

/// `hop.physics.patrol case[0]`.
fn case_35(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::patrol(
            crate::hop::model::Crawler::new(
                1,
                crate::hop::geom::Vec3::new(0, 0, 0)?,
                rt::int::neg(10)?,
                10,
                1,
                4,
            )?,
        ),
    )?;
    let want: crate::hop::model::Crawler = crate::hop::model::Crawler::new(
        1,
        crate::hop::geom::Vec3::new(4, 0, 0)?,
        rt::int::neg(10)?,
        10,
        1,
        4,
    )?;
    Ok(got == Ok(want))
}

/// `hop.physics.patrol case[1]`.
fn case_36(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::physics::patrol(
            crate::hop::model::Crawler::new(
                1,
                crate::hop::geom::Vec3::new(8, 0, 0)?,
                rt::int::neg(10)?,
                10,
                1,
                4,
            )?,
        ),
    )?;
    let want: crate::hop::model::Crawler = crate::hop::model::Crawler::new(
        1,
        crate::hop::geom::Vec3::new(10, 0, 0)?,
        rt::int::neg(10)?,
        10,
        rt::int::neg(1)?,
        4,
    )?;
    Ok(got == Ok(want))
}

/// `hop.render.asset_name case[0]`.
fn case_37(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::render::asset_name(crate::hop::render::Asset::PipMesh),
    )?;
    let want: String = String::from("pip");
    Ok(got == Ok(want))
}

/// `hop.render.asset_name case[1]`.
fn case_38(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::render::asset_name(crate::hop::render::Asset::CrawlerMesh),
    )?;
    let want: String = String::from("crawler");
    Ok(got == Ok(want))
}

/// `hop.render.asset_name case[2]`.
fn case_39(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::render::asset_name(crate::hop::render::Asset::GemMesh),
    )?;
    let want: String = String::from("gem");
    Ok(got == Ok(want))
}

/// `hop.render.asset_name case[3]`.
fn case_40(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::render::asset_name(crate::hop::render::Asset::FlagMesh),
    )?;
    let want: String = String::from("flag");
    Ok(got == Ok(want))
}

/// `hop.render.bob case[0]`.
fn case_41(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::render::bob(0))?;
    let want: Int = rt::int::neg(32)?;
    Ok(got == Ok(want))
}

/// `hop.render.bob case[1]`.
fn case_42(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::render::bob(16))?;
    let want: Int = 0;
    Ok(got == Ok(want))
}

/// `hop.render.bob case[2]`.
fn case_43(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::render::bob(32))?;
    let want: Int = 32;
    Ok(got == Ok(want))
}

/// `hop.render.bob case[3]`.
fn case_44(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::render::bob(48))?;
    let want: Int = 0;
    Ok(got == Ok(want))
}

/// `hop.render.draw_text case[0]`.
fn case_45(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::render::draw_text(
            crate::hop::render::Draw::new_cube(
                crate::hop::geom::Box::new(
                    crate::hop::geom::Vec3::new(0, 0, 0)?,
                    crate::hop::geom::Vec3::new(64, 64, 64)?,
                )?,
                crate::hop::model::Rgb::new(1, 2, 3)?,
            )?,
        ),
    )?;
    let want: String = String::from("cube 0,0,0 64,64,64 1,2,3");
    Ok(got == Ok(want))
}

/// `hop.render.draw_text case[1]`.
fn case_46(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(
        crate::hop::render::draw_text(
            crate::hop::render::Draw::new_mesh(
                crate::hop::render::Asset::PipMesh,
                crate::hop::geom::Vec3::new(1, 2, 3)?,
                90,
            )?,
        ),
    )?;
    let want: String = String::from("mesh pip 1,2,3 90");
    Ok(got == Ok(want))
}

/// Arguments of `hop.tests.edge.touch_matches_raylib` drawn from its parameters' types.
fn draw_47(m: &mut Machine, rng: &mut Rng) -> R<Body> {
    let pools = Pools::default();
    let a0 = crate::draw::r#box(m, rng, &pools)?;
    let a1 = crate::draw::r#box(m, rng, &pools)?;
    Ok(
        Box::new(move |m: &mut Machine| crate::hop::tests::edge::touch_matches_raylib(
            m,
            a0,
            a1,
        )),
    )
}

/// `hop.tests.runs.pad_of case[0]`.
fn case_61(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::tests::runs::pad_of(String::from("-")))?;
    let want: crate::hop::model::Pad = crate::hop::model::Pad::new(
        None,
        None,
        None,
        None,
        None,
    )?;
    Ok(got == Ok(want))
}

/// `hop.tests.runs.pad_of case[1]`.
fn case_62(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::tests::runs::pad_of(String::from("RJ")))?;
    let want: crate::hop::model::Pad = crate::hop::model::Pad::new(
        None,
        Some(true),
        None,
        None,
        Some(true),
    )?;
    Ok(got == Ok(want))
}

/// `hop.tests.runs.pad_of case[2]`.
fn case_63(_m: &mut Machine) -> R<bool> {
    let got = rt::settle(crate::hop::tests::runs::pad_of(String::from("LFB")))?;
    let want: crate::hop::model::Pad = crate::hop::model::Pad::new(
        Some(true),
        None,
        Some(true),
        Some(true),
        None,
    )?;
    Ok(got == Ok(want))
}

/// Every test, by number.
pub static TESTS: &[TestDecl] = &[
    TestDecl {
        name: "hop.tests.edge.touch_matches_raylib",
        has_params: true,
        given: &[],
        run: no_end,
    },
    TestDecl {
        name: "hop.tests.moves.standing_still_stays_put",
        has_params: false,
        given: &[],
        run: crate::hop::tests::moves::standing_still_stays_put,
    },
    TestDecl {
        name: "hop.tests.moves.running_reaches_full_speed",
        has_params: false,
        given: &[],
        run: crate::hop::tests::moves::running_reaches_full_speed,
    },
    TestDecl {
        name: "hop.tests.moves.a_held_jump_peaks_at_380",
        has_params: false,
        given: &[],
        run: crate::hop::tests::moves::a_held_jump_peaks_at_380,
    },
    TestDecl {
        name: "hop.tests.moves.a_short_hop_peaks_at_94",
        has_params: false,
        given: &[],
        run: crate::hop::tests::moves::a_short_hop_peaks_at_94,
    },
    TestDecl {
        name: "hop.tests.moves.a_jump_needs_a_fresh_press",
        has_params: false,
        given: &[],
        run: crate::hop::tests::moves::a_jump_needs_a_fresh_press,
    },
    TestDecl {
        name: "hop.tests.moves.a_jump_just_after_the_edge_still_counts",
        has_params: false,
        given: &[Given::Test(25)],
        run: crate::hop::tests::moves::a_jump_just_after_the_edge_still_counts,
    },
    TestDecl {
        name: "hop.tests.moves.a_jump_long_after_the_edge_does_not",
        has_params: false,
        given: &[Given::Test(25)],
        run: crate::hop::tests::moves::a_jump_long_after_the_edge_does_not,
    },
    TestDecl {
        name: "hop.tests.moves.a_wall_stops_the_body_flush",
        has_params: false,
        given: &[Given::Test(26)],
        run: crate::hop::tests::moves::a_wall_stops_the_body_flush,
    },
    TestDecl {
        name: "hop.tests.moves.a_ceiling_stops_a_jump",
        has_params: false,
        given: &[Given::Test(27)],
        run: crate::hop::tests::moves::a_ceiling_stops_a_jump,
    },
    TestDecl {
        name: "hop.tests.moves.a_rider_moves_with_the_mover",
        has_params: false,
        given: &[Given::Test(28)],
        run: crate::hop::tests::moves::a_rider_moves_with_the_mover,
    },
    TestDecl {
        name: "hop.tests.moves.a_mover_catches_a_falling_body",
        has_params: false,
        given: &[Given::Test(29)],
        run: crate::hop::tests::moves::a_mover_catches_a_falling_body,
    },
    TestDecl {
        name: "hop.tests.moves.a_mover_never_catches_from_below",
        has_params: false,
        given: &[Given::Test(38)],
        run: crate::hop::tests::moves::a_mover_never_catches_from_below,
    },
    TestDecl {
        name: "hop.tests.moves.a_fast_fall_lands",
        has_params: false,
        given: &[Given::Test(39)],
        run: crate::hop::tests::moves::a_fast_fall_lands,
    },
    TestDecl {
        name: "hop.tests.runs.the_meadow_can_be_won",
        has_params: false,
        given: &[],
        run: crate::hop::tests::runs::the_meadow_can_be_won,
    },
    TestDecl {
        name: "hop.tests.runs.the_ridge_can_be_won",
        has_params: false,
        given: &[],
        run: crate::hop::tests::runs::the_ridge_can_be_won,
    },
    TestDecl {
        name: "hop.tests.touches.a_gem_is_taken_once",
        has_params: false,
        given: &[Given::Test(30)],
        run: crate::hop::tests::touches::a_gem_is_taken_once,
    },
    TestDecl {
        name: "hop.tests.touches.a_fall_keeps_the_gems",
        has_params: false,
        given: &[Given::Test(36)],
        run: crate::hop::tests::touches::a_fall_keeps_the_gems,
    },
    TestDecl {
        name: "hop.tests.touches.rising_into_a_crawler_hurts",
        has_params: false,
        given: &[Given::Test(37)],
        run: crate::hop::tests::touches::rising_into_a_crawler_hurts,
    },
    TestDecl {
        name: "hop.tests.touches.a_stomp_removes_the_crawler",
        has_params: false,
        given: &[Given::Test(31)],
        run: crate::hop::tests::touches::a_stomp_removes_the_crawler,
    },
    TestDecl {
        name: "hop.tests.touches.a_crawler_hurts_from_the_side",
        has_params: false,
        given: &[Given::Test(32)],
        run: crate::hop::tests::touches::a_crawler_hurts_from_the_side,
    },
    TestDecl {
        name: "hop.tests.touches.falling_out_costs_a_life",
        has_params: false,
        given: &[Given::Test(33)],
        run: crate::hop::tests::touches::falling_out_costs_a_life,
    },
    TestDecl {
        name: "hop.tests.touches.the_last_life_ends_the_game",
        has_params: false,
        given: &[Given::Test(34)],
        run: crate::hop::tests::touches::the_last_life_ends_the_game,
    },
    TestDecl {
        name: "hop.tests.touches.reaching_the_flag_wins",
        has_params: false,
        given: &[Given::Test(35)],
        run: crate::hop::tests::touches::reaching_the_flag_wins,
    },
    TestDecl {
        name: "hop.tests.touches.starting_again_resets_everything",
        has_params: false,
        given: &[],
        run: crate::hop::tests::touches::starting_again_resets_everything,
    },
    TestDecl {
        name: "hop.world.past_the_first_ledge",
        has_params: false,
        given: &[Given::State(given_25_0), Given::State(given_25_1)],
        run: crate::hop::world::past_the_first_ledge,
    },
    TestDecl {
        name: "hop.world.running_at_the_step",
        has_params: false,
        given: &[Given::State(given_26_0), Given::State(given_26_1)],
        run: crate::hop::world::running_at_the_step,
    },
    TestDecl {
        name: "hop.world.under_a_low_ceiling",
        has_params: false,
        given: &[Given::State(given_27_0), Given::State(given_27_1)],
        run: crate::hop::world::under_a_low_ceiling,
    },
    TestDecl {
        name: "hop.world.riding_the_mover",
        has_params: false,
        given: &[Given::State(given_28_0), Given::State(given_28_1)],
        run: crate::hop::world::riding_the_mover,
    },
    TestDecl {
        name: "hop.world.above_the_mover",
        has_params: false,
        given: &[Given::State(given_29_0), Given::State(given_29_1)],
        run: crate::hop::world::above_the_mover,
    },
    TestDecl {
        name: "hop.world.at_the_third_gem",
        has_params: false,
        given: &[Given::State(given_30_0), Given::State(given_30_1)],
        run: crate::hop::world::at_the_third_gem,
    },
    TestDecl {
        name: "hop.world.falling_on_the_crawler",
        has_params: false,
        given: &[
            Given::State(given_31_0),
            Given::State(given_31_1),
            Given::State(given_31_2),
        ],
        run: crate::hop::world::falling_on_the_crawler,
    },
    TestDecl {
        name: "hop.world.running_at_the_crawler",
        has_params: false,
        given: &[
            Given::State(given_32_0),
            Given::State(given_32_1),
            Given::State(given_32_2),
        ],
        run: crate::hop::world::running_at_the_crawler,
    },
    TestDecl {
        name: "hop.world.about_to_fall_out",
        has_params: false,
        given: &[Given::State(given_33_0), Given::State(given_33_1)],
        run: crate::hop::world::about_to_fall_out,
    },
    TestDecl {
        name: "hop.world.about_to_fall_out_for_the_last_time",
        has_params: false,
        given: &[
            Given::State(given_34_0),
            Given::State(given_34_1),
            Given::State(given_34_2),
        ],
        run: crate::hop::world::about_to_fall_out_for_the_last_time,
    },
    TestDecl {
        name: "hop.world.beside_the_flag",
        has_params: false,
        given: &[Given::State(given_35_0), Given::State(given_35_1)],
        run: crate::hop::world::beside_the_flag,
    },
    TestDecl {
        name: "hop.world.about_to_fall_out_holding_a_gem",
        has_params: false,
        given: &[
            Given::State(given_36_0),
            Given::State(given_36_1),
            Given::State(given_36_2),
        ],
        run: crate::hop::world::about_to_fall_out_holding_a_gem,
    },
    TestDecl {
        name: "hop.world.rising_beside_the_crawler",
        has_params: false,
        given: &[
            Given::State(given_37_0),
            Given::State(given_37_1),
            Given::State(given_37_2),
        ],
        run: crate::hop::world::rising_beside_the_crawler,
    },
    TestDecl {
        name: "hop.world.under_the_mover",
        has_params: false,
        given: &[Given::State(given_38_0), Given::State(given_38_1)],
        run: crate::hop::world::under_the_mover,
    },
    TestDecl {
        name: "hop.world.falling_fast_onto_the_floor",
        has_params: false,
        given: &[Given::State(given_39_0), Given::State(given_39_1)],
        run: crate::hop::world::falling_fast_onto_the_floor,
    },
];

/// Every law, by number.
pub static LAWS: &[LawDecl] = &[
    LawDecl {
        name: "hop.laws.never_inside_a_solid",
        traced: &[10, 12],
        start: &[10, 11, 12],
        draw: law_0,
    },
    LawDecl {
        name: "hop.laws.speed_is_bounded",
        traced: &[10, 12],
        start: &[10, 11, 12],
        draw: law_1,
    },
    LawDecl {
        name: "hop.laws.taken_only_grows",
        traced: &[12],
        start: &[10, 11, 12],
        draw: law_2,
    },
    LawDecl {
        name: "hop.laws.lives_never_rise",
        traced: &[12],
        start: &[10, 11, 12],
        draw: law_3,
    },
    LawDecl {
        name: "hop.laws.lost_means_no_lives",
        traced: &[10, 12],
        start: &[10, 11, 12],
        draw: law_4,
    },
    LawDecl {
        name: "hop.laws.over_is_final",
        traced: &[12],
        start: &[10, 11, 12],
        draw: law_5,
    },
    LawDecl {
        name: "hop.laws.one_tick_per_step",
        traced: &[12],
        start: &[10, 11, 12],
        draw: law_6,
    },
    LawDecl {
        name: "hop.laws.taken_are_real_gems",
        traced: &[10, 12],
        start: &[10, 11, 12],
        draw: law_7,
    },
];

/// Every check, in the order `spec-lang test` runs them.
pub static CHECKS: &[Check] = &[
    Check {
        name: "hop.geom.plus case[0]",
        what: What::Case(case_0),
    },
    Check {
        name: "hop.geom.around_feet case[0]",
        what: What::Case(case_1),
    },
    Check {
        name: "hop.geom.overlaps case[0]",
        what: What::Case(case_2),
    },
    Check {
        name: "hop.geom.overlaps case[1]",
        what: What::Case(case_3),
    },
    Check {
        name: "hop.geom.touches case[0]",
        what: What::Case(case_4),
    },
    Check {
        name: "hop.geom.touches case[1]",
        what: What::Case(case_5),
    },
    Check {
        name: "hop.geom.triangle case[0]",
        what: What::Case(case_6),
    },
    Check {
        name: "hop.geom.triangle case[1]",
        what: What::Case(case_7),
    },
    Check {
        name: "hop.geom.triangle case[2]",
        what: What::Case(case_8),
    },
    Check {
        name: "hop.geom.triangle case[3]",
        what: What::Case(case_9),
    },
    Check {
        name: "hop.geom.triangle case[4]",
        what: What::Case(case_10),
    },
    Check {
        name: "hop.geom.triangle case[5]",
        what: What::Case(case_11),
    },
    Check {
        name: "hop.geom.approach case[0]",
        what: What::Case(case_12),
    },
    Check {
        name: "hop.geom.approach case[1]",
        what: What::Case(case_13),
    },
    Check {
        name: "hop.geom.approach case[2]",
        what: What::Case(case_14),
    },
    Check {
        name: "hop.geom.approach case[3]",
        what: What::Case(case_15),
    },
    Check {
        name: "hop.geom.vec_text case[0]",
        what: What::Case(case_16),
    },
    Check {
        name: "hop.laws.never_inside_a_solid",
        what: What::Law(0),
    },
    Check {
        name: "hop.laws.speed_is_bounded",
        what: What::Law(1),
    },
    Check {
        name: "hop.laws.taken_only_grows",
        what: What::Law(2),
    },
    Check {
        name: "hop.laws.lives_never_rise",
        what: What::Law(3),
    },
    Check {
        name: "hop.laws.lost_means_no_lives",
        what: What::Law(4),
    },
    Check {
        name: "hop.laws.over_is_final",
        what: What::Law(5),
    },
    Check {
        name: "hop.laws.one_tick_per_step",
        what: What::Law(6),
    },
    Check {
        name: "hop.laws.taken_are_real_gems",
        what: What::Law(7),
    },
    Check {
        name: "hop.levels.q case[0]",
        what: What::Case(case_25),
    },
    Check {
        name: "hop.levels.q case[1]",
        what: What::Case(case_26),
    },
    Check {
        name: "hop.physics.walk_target case[0]",
        what: What::Case(case_27),
    },
    Check {
        name: "hop.physics.walk_target case[1]",
        what: What::Case(case_28),
    },
    Check {
        name: "hop.physics.walk_target case[2]",
        what: What::Case(case_29),
    },
    Check {
        name: "hop.physics.walk_target case[3]",
        what: What::Case(case_30),
    },
    Check {
        name: "hop.physics.facing case[0]",
        what: What::Case(case_31),
    },
    Check {
        name: "hop.physics.facing case[1]",
        what: What::Case(case_32),
    },
    Check {
        name: "hop.physics.facing case[2]",
        what: What::Case(case_33),
    },
    Check {
        name: "hop.physics.facing case[3]",
        what: What::Case(case_34),
    },
    Check {
        name: "hop.physics.patrol case[0]",
        what: What::Case(case_35),
    },
    Check {
        name: "hop.physics.patrol case[1]",
        what: What::Case(case_36),
    },
    Check {
        name: "hop.render.asset_name case[0]",
        what: What::Case(case_37),
    },
    Check {
        name: "hop.render.asset_name case[1]",
        what: What::Case(case_38),
    },
    Check {
        name: "hop.render.asset_name case[2]",
        what: What::Case(case_39),
    },
    Check {
        name: "hop.render.asset_name case[3]",
        what: What::Case(case_40),
    },
    Check {
        name: "hop.render.bob case[0]",
        what: What::Case(case_41),
    },
    Check {
        name: "hop.render.bob case[1]",
        what: What::Case(case_42),
    },
    Check {
        name: "hop.render.bob case[2]",
        what: What::Case(case_43),
    },
    Check {
        name: "hop.render.bob case[3]",
        what: What::Case(case_44),
    },
    Check {
        name: "hop.render.draw_text case[0]",
        what: What::Case(case_45),
    },
    Check {
        name: "hop.render.draw_text case[1]",
        what: What::Case(case_46),
    },
    Check {
        name: "hop.tests.edge.touch_matches_raylib",
        what: What::Property {
            test: 0,
            draw: draw_47,
        },
    },
    Check {
        name: "hop.tests.moves.standing_still_stays_put",
        what: What::Test(1),
    },
    Check {
        name: "hop.tests.moves.running_reaches_full_speed",
        what: What::Test(2),
    },
    Check {
        name: "hop.tests.moves.a_held_jump_peaks_at_380",
        what: What::Test(3),
    },
    Check {
        name: "hop.tests.moves.a_short_hop_peaks_at_94",
        what: What::Test(4),
    },
    Check {
        name: "hop.tests.moves.a_jump_needs_a_fresh_press",
        what: What::Test(5),
    },
    Check {
        name: "hop.tests.moves.a_jump_just_after_the_edge_still_counts",
        what: What::Test(6),
    },
    Check {
        name: "hop.tests.moves.a_jump_long_after_the_edge_does_not",
        what: What::Test(7),
    },
    Check {
        name: "hop.tests.moves.a_wall_stops_the_body_flush",
        what: What::Test(8),
    },
    Check {
        name: "hop.tests.moves.a_ceiling_stops_a_jump",
        what: What::Test(9),
    },
    Check {
        name: "hop.tests.moves.a_rider_moves_with_the_mover",
        what: What::Test(10),
    },
    Check {
        name: "hop.tests.moves.a_mover_catches_a_falling_body",
        what: What::Test(11),
    },
    Check {
        name: "hop.tests.moves.a_mover_never_catches_from_below",
        what: What::Test(12),
    },
    Check {
        name: "hop.tests.moves.a_fast_fall_lands",
        what: What::Test(13),
    },
    Check {
        name: "hop.tests.runs.pad_of case[0]",
        what: What::Case(case_61),
    },
    Check {
        name: "hop.tests.runs.pad_of case[1]",
        what: What::Case(case_62),
    },
    Check {
        name: "hop.tests.runs.pad_of case[2]",
        what: What::Case(case_63),
    },
    Check {
        name: "hop.tests.runs.the_meadow_can_be_won",
        what: What::Test(14),
    },
    Check {
        name: "hop.tests.runs.the_ridge_can_be_won",
        what: What::Test(15),
    },
    Check {
        name: "hop.tests.touches.a_gem_is_taken_once",
        what: What::Test(16),
    },
    Check {
        name: "hop.tests.touches.a_fall_keeps_the_gems",
        what: What::Test(17),
    },
    Check {
        name: "hop.tests.touches.rising_into_a_crawler_hurts",
        what: What::Test(18),
    },
    Check {
        name: "hop.tests.touches.a_stomp_removes_the_crawler",
        what: What::Test(19),
    },
    Check {
        name: "hop.tests.touches.a_crawler_hurts_from_the_side",
        what: What::Test(20),
    },
    Check {
        name: "hop.tests.touches.falling_out_costs_a_life",
        what: What::Test(21),
    },
    Check {
        name: "hop.tests.touches.the_last_life_ends_the_game",
        what: What::Test(22),
    },
    Check {
        name: "hop.tests.touches.reaching_the_flag_wins",
        what: What::Test(23),
    },
    Check {
        name: "hop.tests.touches.starting_again_resets_everything",
        what: What::Test(24),
    },
    Check {
        name: "hop.world.past_the_first_ledge",
        what: What::Test(25),
    },
    Check {
        name: "hop.world.running_at_the_step",
        what: What::Test(26),
    },
    Check {
        name: "hop.world.under_a_low_ceiling",
        what: What::Test(27),
    },
    Check {
        name: "hop.world.riding_the_mover",
        what: What::Test(28),
    },
    Check {
        name: "hop.world.above_the_mover",
        what: What::Test(29),
    },
    Check {
        name: "hop.world.at_the_third_gem",
        what: What::Test(30),
    },
    Check {
        name: "hop.world.falling_on_the_crawler",
        what: What::Test(31),
    },
    Check {
        name: "hop.world.running_at_the_crawler",
        what: What::Test(32),
    },
    Check {
        name: "hop.world.about_to_fall_out",
        what: What::Test(33),
    },
    Check {
        name: "hop.world.about_to_fall_out_for_the_last_time",
        what: What::Test(34),
    },
    Check {
        name: "hop.world.beside_the_flag",
        what: What::Test(35),
    },
    Check {
        name: "hop.world.about_to_fall_out_holding_a_gem",
        what: What::Test(36),
    },
    Check {
        name: "hop.world.rising_beside_the_crawler",
        what: What::Test(37),
    },
    Check {
        name: "hop.world.under_the_mover",
        what: What::Test(38),
    },
    Check {
        name: "hop.world.falling_fast_onto_the_floor",
        what: What::Test(39),
    },
];
