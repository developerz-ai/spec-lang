//! Module `hop.levels`, printed by spec-lang from `src/hop/levels.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R};

/// Type `hop.levels.LevelNo`, `src/hop/levels.slg:12`.
/// Checked against `1..=2` wherever a value of it is given.
pub type LevelNo = Int;

/// Constant `hop.levels.grass`, `src/hop/levels.slg:14`.
pub fn grass() -> R<crate::hop::model::Rgb> {
    crate::hop::model::Rgb::new(98, 170, 80)
}

/// Constant `hop.levels.stone`, `src/hop/levels.slg:15`.
pub fn stone() -> R<crate::hop::model::Rgb> {
    crate::hop::model::Rgb::new(150, 150, 160)
}

/// Constant `hop.levels.wood`, `src/hop/levels.slg:16`.
pub fn wood() -> R<crate::hop::model::Rgb> {
    crate::hop::model::Rgb::new(210, 150, 60)
}

/// teaches running, a gap, a step up, a crawler and the flag
///
/// Constant `hop.levels.meadow`, `src/hop/levels.slg:26`.
pub fn meadow() -> R<crate::hop::model::Level> {
    crate::hop::model::Level::new(
        String::from("meadow"),
        at(0, 0, 0)?,
        vec![
            slab(
                at(rt::int::neg(8)?, rt::int::neg(4)?, rt::int::neg(8)?)?,
                at(16, 0, 8)?,
                grass()?,
            )?,
            slab(at(24, rt::int::neg(4)?, rt::int::neg(8)?)?, at(40, 0, 8)?, grass()?)?,
            slab(at(40, rt::int::neg(4)?, rt::int::neg(8)?)?, at(56, 4, 8)?, stone()?)?,
            slab(at(64, rt::int::neg(4)?, rt::int::neg(8)?)?, at(84, 0, 8)?, grass()?)?,
        ],
        None,
        Some(
            vec![
                crate::hop::model::Gem::new(1, at(32, 6, 0)?)?,
                crate::hop::model::Gem::new(2, at(60, 10, 0)?)?,
                crate::hop::model::Gem::new(3, at(72, 2, 0)?)?,
            ],
        ),
        Some(
            vec![
                crate::hop::model::Crawler::new(1, at(48, 4, 0)?, q(43)?, q(53)?, 1, 2)?,
            ],
        ),
        at(80, 0, 0)?,
    )
}

/// a moving platform over a wide gap, then two crawlers on high ground a jump can reach
///
/// Constant `hop.levels.ridge`, `src/hop/levels.slg:41`.
pub fn ridge() -> R<crate::hop::model::Level> {
    crate::hop::model::Level::new(
        String::from("ridge"),
        at(0, 0, 0)?,
        vec![
            slab(
                at(rt::int::neg(8)?, rt::int::neg(4)?, rt::int::neg(8)?)?,
                at(16, 0, 8)?,
                grass()?,
            )?,
            slab(at(52, rt::int::neg(4)?, rt::int::neg(8)?)?, at(60, 0, 8)?, grass()?)?,
            slab(at(60, rt::int::neg(4)?, rt::int::neg(8)?)?, at(76, 5, 8)?, stone()?)?,
            slab(at(80, rt::int::neg(4)?, rt::int::neg(8)?)?, at(100, 5, 8)?, grass()?)?,
        ],
        Some(
            vec![
                crate::hop::model::Mover::new(
                    1,
                    crate::hop::geom::Box::new(
                        at(20, rt::int::neg(2)?, rt::int::neg(6)?)?,
                        at(32, 0, 6)?,
                    )?,
                    crate::hop::geom::Vec3::new(8, 0, 0)?,
                    128,
                    wood()?,
                )?,
            ],
        ),
        Some(
            vec![
                crate::hop::model::Gem::new(1, at(34, 5, 0)?)?,
                crate::hop::model::Gem::new(2, at(68, 9, 0)?)?,
                crate::hop::model::Gem::new(3, at(90, 9, 0)?)?,
            ],
        ),
        Some(
            vec![
                crate::hop::model::Crawler::new(1, at(68, 5, 0)?, q(62)?, q(74)?, 1, 3)?,
                crate::hop::model::Crawler::new(
                    2,
                    at(88, 5, 0)?,
                    q(82)?,
                    q(94)?,
                    rt::int::neg(1)?,
                    2,
                )?,
            ],
        ),
        at(96, 5, 0)?,
    )
}

/// levels are laid out on a quarter-metre grid: 64 sub-metres
///
/// Fn `hop.levels.q`, `src/hop/levels.slg:18`.
pub fn q(n: Int) -> R<Int> {
    rt::int::mul(n, 64)
}

/// Fn `hop.levels.at`, `src/hop/levels.slg:22`.
pub fn at(x: Int, y: Int, z: Int) -> R<crate::hop::geom::Vec3> {
    let result: crate::hop::geom::Vec3 = crate::hop::geom::Vec3::new(
        q(x)?,
        q(y)?,
        q(z)?,
    )?;
    rt::claim(crate::conform::vec3(&result)?, "the result of hop.levels.at")?;
    Ok(result)
}

/// Fn `hop.levels.slab`, `src/hop/levels.slg:24`.
pub fn slab(
    from: crate::hop::geom::Vec3,
    to: crate::hop::geom::Vec3,
    color: crate::hop::model::Rgb,
) -> R<crate::hop::model::Solid> {
    rt::claim(crate::conform::vec3(&from)?, "argument `from` of hop.levels.slab")?;
    rt::claim(crate::conform::vec3(&to)?, "argument `to` of hop.levels.slab")?;
    rt::claim(crate::conform::rgb(&color)?, "argument `color` of hop.levels.slab")?;
    let result: crate::hop::model::Solid = crate::hop::model::Solid::new(
        crate::hop::geom::Box::new(from.clone(), to.clone())?,
        color.clone(),
    )?;
    rt::claim(crate::conform::solid(&result)?, "the result of hop.levels.slab")?;
    Ok(result)
}

/// Fn `hop.levels.level`, `src/hop/levels.slg:60`.
pub fn level(n: Int) -> R<crate::hop::model::Level> {
    rt::claim(crate::conform::level_no(&n)?, "argument `n` of hop.levels.level")?;
    let result: crate::hop::model::Level = if n == 1 { meadow()? } else { ridge()? };
    rt::claim(crate::conform::level(&result)?, "the result of hop.levels.level")?;
    Ok(result)
}
