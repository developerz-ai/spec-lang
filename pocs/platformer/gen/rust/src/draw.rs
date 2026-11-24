//! A generator per declared type a property, law or trace draws from: the engine's draws.

use crate::rt::{self, Int, Opt, R};
use crate::rt::rng::Rng;
use crate::machine::{Machine, Pools};
use std::collections::{BTreeMap, BTreeSet};

/// A value of `hop.levels.LevelNo`.
pub fn level_no(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = range_1_2(m, rng, pools)?;
        if crate::conform::level_no(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.levels.LevelNo` found in 400 tries"))
}

/// A value of `Text`.
pub fn text(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<String> {
    Ok(rt::rng::text(rng))
}

/// A value of `hop.model.Pad`.
pub fn pad(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<crate::hop::model::Pad> {
    Ok(crate::hop::model::Pad {
        left: bool(m, rng, pools)?,
        right: bool(m, rng, pools)?,
        forward: bool(m, rng, pools)?,
        back: bool(m, rng, pools)?,
        jump: bool(m, rng, pools)?,
    })
}

/// A value of `hop.model.Level`.
pub fn level(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<crate::hop::model::Level> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::model::Level {
            name: text(m, rng, pools)?,
            start: vec3(m, rng, pools)?,
            solids: list_solid(m, rng, pools)?,
            movers: list_mover(m, rng, pools)?,
            gems: list_gem(m, rng, pools)?,
            crawlers: list_crawler(m, rng, pools)?,
            goal: vec3(m, rng, pools)?,
        };
        if crate::conform::level(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.model.Level` found in 400 tries"))
}

/// A value of `Int`.
pub fn int(_m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<Int> {
    Ok(
        match rt::rng::from_pool(rng, &pools.p0) {
            Some(v) => v,
            None => rt::rng::int(rng),
        },
    )
}

/// A value of `hop.model.Player`.
pub fn player(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<crate::hop::model::Player> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::model::Player {
            at: vec3(m, rng, pools)?,
            vel: vec3(m, rng, pools)?,
            grounded: bool(m, rng, pools)?,
            coyote: range_0_6(m, rng, pools)?,
            jump_held: bool(m, rng, pools)?,
            riding: option_int(m, rng, pools)?,
            yaw: range_0_359(m, rng, pools)?,
        };
        if crate::conform::player(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.model.Player` found in 400 tries"))
}

/// A value of `Set[Int]`.
pub fn set_int(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<BTreeSet<Int>> {
    Ok({
        let n = rng.below(4);
        let mut xs = BTreeSet::new();
        for _ in 0..n {
            xs.insert(int(m, rng, pools)?);
        }
        xs
    })
}

/// A value of `Map[Int, hop.model.Crawler]`.
pub fn map_int_crawler(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<BTreeMap<Int, crate::hop::model::Crawler>> {
    for _ in 0..rt::rng::TRIES {
        let v = {
            let n = rng.below(4);
            let mut map = BTreeMap::new();
            for _ in 0..n {
                let k = int(m, rng, pools)?;
                let v = crawler(m, rng, pools)?;
                map.insert(k, v);
            }
            map
        };
        if crate::conform::map_int_crawler(&v)? {
            return Ok(v);
        }
    }
    Err(
        rt::stop("E-GEN", "no value of `Map[Int, hop.model.Crawler]` found in 400 tries"),
    )
}

/// A value of `0..=3`.
pub fn range_0_3(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, 0, 3);
        if crate::conform::range_0_3(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `0..=3` found in 400 tries"))
}

/// A value of `hop.model.Mode`.
pub fn mode(
    _m: &mut Machine,
    rng: &mut Rng,
    _pools: &Pools,
) -> R<crate::hop::model::Mode> {
    Ok(
        match rng.below(3) {
            0 => crate::hop::model::Mode::Playing,
            1 => crate::hop::model::Mode::Won,
            _ => crate::hop::model::Mode::Lost,
        },
    )
}

/// A value of `hop.geom.Box`.
pub fn r#box(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<crate::hop::geom::Box> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::geom::Box {
            lo: vec3(m, rng, pools)?,
            hi: vec3(m, rng, pools)?,
        };
        if crate::conform::r#box(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.geom.Box` found in 400 tries"))
}

/// A value of `1..=2`.
pub fn range_1_2(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, 1, 2);
        if crate::conform::range_1_2(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `1..=2` found in 400 tries"))
}

/// A value of `Bool`.
pub fn bool(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<bool> {
    Ok(rng.chance(1, 2))
}

/// A value of `hop.geom.Vec3`.
pub fn vec3(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<crate::hop::geom::Vec3> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::geom::Vec3 {
            x: sub(m, rng, pools)?,
            y: sub(m, rng, pools)?,
            z: sub(m, rng, pools)?,
        };
        if crate::conform::vec3(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.geom.Vec3` found in 400 tries"))
}

/// A value of `List[hop.model.Solid]`.
pub fn list_solid(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<Vec<crate::hop::model::Solid>> {
    for _ in 0..rt::rng::TRIES {
        let v = {
            let n = rng.below(4);
            let mut xs = Vec::new();
            for _ in 0..n {
                xs.push(solid(m, rng, pools)?);
            }
            xs
        };
        if crate::conform::list_solid(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `List[hop.model.Solid]` found in 400 tries"))
}

/// A value of `List[hop.model.Mover]`.
pub fn list_mover(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<Vec<crate::hop::model::Mover>> {
    for _ in 0..rt::rng::TRIES {
        let v = {
            let n = rng.below(4);
            let mut xs = Vec::new();
            for _ in 0..n {
                xs.push(mover(m, rng, pools)?);
            }
            xs
        };
        if crate::conform::list_mover(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `List[hop.model.Mover]` found in 400 tries"))
}

/// A value of `List[hop.model.Gem]`.
pub fn list_gem(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<Vec<crate::hop::model::Gem>> {
    for _ in 0..rt::rng::TRIES {
        let v = {
            let n = rng.below(4);
            let mut xs = Vec::new();
            for _ in 0..n {
                xs.push(gem(m, rng, pools)?);
            }
            xs
        };
        if crate::conform::list_gem(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `List[hop.model.Gem]` found in 400 tries"))
}

/// A value of `List[hop.model.Crawler]`.
pub fn list_crawler(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<Vec<crate::hop::model::Crawler>> {
    for _ in 0..rt::rng::TRIES {
        let v = {
            let n = rng.below(4);
            let mut xs = Vec::new();
            for _ in 0..n {
                xs.push(crawler(m, rng, pools)?);
            }
            xs
        };
        if crate::conform::list_crawler(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `List[hop.model.Crawler]` found in 400 tries"))
}

/// A value of `0..=6`.
pub fn range_0_6(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, 0, 6);
        if crate::conform::range_0_6(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `0..=6` found in 400 tries"))
}

/// A value of `Option[Int]`.
pub fn option_int(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<Opt<Int>> {
    Ok(if rng.chance(1, 3) { Opt::None } else { Opt::Some(int(m, rng, pools)?) })
}

/// A value of `0..=359`.
pub fn range_0_359(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, 0, 359);
        if crate::conform::range_0_359(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `0..=359` found in 400 tries"))
}

/// A value of `hop.model.Crawler`.
pub fn crawler(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<crate::hop::model::Crawler> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::model::Crawler {
            no: int(m, rng, pools)?,
            at: vec3(m, rng, pools)?,
            lo: int(m, rng, pools)?,
            hi: int(m, rng, pools)?,
            dir: range_minus1_1(m, rng, pools)?,
            speed: range_1_8(m, rng, pools)?,
        };
        if crate::conform::crawler(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.model.Crawler` found in 400 tries"))
}

/// A value of `hop.geom.Sub`.
pub fn sub(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = range_minus1048576_1048576(m, rng, pools)?;
        if crate::conform::sub(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.geom.Sub` found in 400 tries"))
}

/// A value of `hop.model.Solid`.
pub fn solid(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<crate::hop::model::Solid> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::model::Solid {
            r#box: r#box(m, rng, pools)?,
            color: rgb(m, rng, pools)?,
        };
        if crate::conform::solid(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.model.Solid` found in 400 tries"))
}

/// A value of `hop.model.Mover`.
pub fn mover(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<crate::hop::model::Mover> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::model::Mover {
            no: int(m, rng, pools)?,
            home: r#box(m, rng, pools)?,
            step: vec3(m, rng, pools)?,
            span: range_1_1024(m, rng, pools)?,
            color: rgb(m, rng, pools)?,
        };
        if crate::conform::mover(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.model.Mover` found in 400 tries"))
}

/// A value of `hop.model.Gem`.
pub fn gem(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<crate::hop::model::Gem> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::model::Gem {
            no: int(m, rng, pools)?,
            at: vec3(m, rng, pools)?,
        };
        if crate::conform::gem(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.model.Gem` found in 400 tries"))
}

/// A value of `-1..=1`.
pub fn range_minus1_1(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, -1, 1);
        if crate::conform::range_minus1_1(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `-1..=1` found in 400 tries"))
}

/// A value of `1..=8`.
pub fn range_1_8(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, 1, 8);
        if crate::conform::range_1_8(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `1..=8` found in 400 tries"))
}

/// A value of `-1048576..=1048576`.
pub fn range_minus1048576_1048576(
    _m: &mut Machine,
    rng: &mut Rng,
    _pools: &Pools,
) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, -1048576, 1048576);
        if crate::conform::range_minus1048576_1048576(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `-1048576..=1048576` found in 400 tries"))
}

/// A value of `hop.model.Rgb`.
pub fn rgb(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<crate::hop::model::Rgb> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::hop::model::Rgb {
            r: range_0_255(m, rng, pools)?,
            g: range_0_255(m, rng, pools)?,
            b: range_0_255(m, rng, pools)?,
        };
        if crate::conform::rgb(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `hop.model.Rgb` found in 400 tries"))
}

/// A value of `1..=1024`.
pub fn range_1_1024(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, 1, 1024);
        if crate::conform::range_1_1024(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `1..=1024` found in 400 tries"))
}

/// A value of `0..=255`.
pub fn range_0_255(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = rt::rng::in_range(rng, 0, 255);
        if crate::conform::range_0_255(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `0..=255` found in 400 tries"))
}
