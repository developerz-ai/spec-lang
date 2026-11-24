//! Whether a value is of a declared type: ranges and `where` claims, field by field.

use crate::rt::{Int, R};
use std::collections::BTreeMap;

/// Whether a value is of `hop.geom.Sub`.
pub fn sub(v: &Int) -> R<bool> {
    range_minus1048576_1048576(v)
}

/// Whether a value is of `hop.geom.Vec3`.
pub fn vec3(v: &crate::hop::geom::Vec3) -> R<bool> {
    if !sub(&v.x)? {
        return Ok(false);
    }
    if !sub(&v.y)? {
        return Ok(false);
    }
    sub(&v.z)
}

/// Whether a value is of `hop.geom.Box`.
pub fn r#box(v: &crate::hop::geom::Box) -> R<bool> {
    if !vec3(&v.lo)? {
        return Ok(false);
    }
    vec3(&v.hi)
}

/// Whether a value is of `hop.model.Rgb`.
pub fn rgb(v: &crate::hop::model::Rgb) -> R<bool> {
    if !range_0_255(&v.r)? {
        return Ok(false);
    }
    if !range_0_255(&v.g)? {
        return Ok(false);
    }
    range_0_255(&v.b)
}

/// Whether a value is of `hop.model.Solid`.
pub fn solid(v: &crate::hop::model::Solid) -> R<bool> {
    if !r#box(&v.r#box)? {
        return Ok(false);
    }
    rgb(&v.color)
}

/// Whether a value is of `hop.levels.LevelNo`.
pub fn level_no(v: &Int) -> R<bool> {
    range_1_2(v)
}

/// Whether a value is of `hop.model.Level`.
pub fn level(v: &crate::hop::model::Level) -> R<bool> {
    if !vec3(&v.start)? {
        return Ok(false);
    }
    if !list_solid(&v.solids)? {
        return Ok(false);
    }
    if !list_mover(&v.movers)? {
        return Ok(false);
    }
    if !list_gem(&v.gems)? {
        return Ok(false);
    }
    if !list_crawler(&v.crawlers)? {
        return Ok(false);
    }
    vec3(&v.goal)
}

/// Whether a value is of `0..=255`.
pub fn range_0_255(v: &Int) -> R<bool> {
    Ok((0..=255).contains(v))
}

/// Whether a value is of `1..=1024`.
pub fn range_1_1024(v: &Int) -> R<bool> {
    Ok((1..=1024).contains(v))
}

/// Whether a value is of `-1..=1`.
pub fn range_minus1_1(v: &Int) -> R<bool> {
    Ok((-1..=1).contains(v))
}

/// Whether a value is of `1..=8`.
pub fn range_1_8(v: &Int) -> R<bool> {
    Ok((1..=8).contains(v))
}

/// Whether a value is of `List[hop.model.Solid]`.
pub fn list_solid(v: &[crate::hop::model::Solid]) -> R<bool> {
    for x in v {
        if !solid(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `List[hop.model.Mover]`.
pub fn list_mover(v: &[crate::hop::model::Mover]) -> R<bool> {
    for x in v {
        if !mover(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `List[hop.model.Gem]`.
pub fn list_gem(v: &[crate::hop::model::Gem]) -> R<bool> {
    for x in v {
        if !gem(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `List[hop.model.Crawler]`.
pub fn list_crawler(v: &[crate::hop::model::Crawler]) -> R<bool> {
    for x in v {
        if !crawler(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `0..=6`.
pub fn range_0_6(v: &Int) -> R<bool> {
    Ok((0..=6).contains(v))
}

/// Whether a value is of `0..=359`.
pub fn range_0_359(v: &Int) -> R<bool> {
    Ok((0..=359).contains(v))
}

/// Whether a value is of `hop.model.Crawler`.
pub fn crawler(v: &crate::hop::model::Crawler) -> R<bool> {
    if !vec3(&v.at)? {
        return Ok(false);
    }
    if !range_minus1_1(&v.dir)? {
        return Ok(false);
    }
    range_1_8(&v.speed)
}

/// Whether a value is of `hop.model.Mover`.
pub fn mover(v: &crate::hop::model::Mover) -> R<bool> {
    if !r#box(&v.home)? {
        return Ok(false);
    }
    if !vec3(&v.step)? {
        return Ok(false);
    }
    if !range_1_1024(&v.span)? {
        return Ok(false);
    }
    rgb(&v.color)
}

/// Whether a value is of `List[hop.physics.Placed]`.
pub fn list_placed(v: &[crate::hop::physics::Placed]) -> R<bool> {
    for x in v {
        if !placed(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `List[hop.render.Draw]`.
pub fn list_draw(v: &[crate::hop::render::Draw]) -> R<bool> {
    for x in v {
        if !draw(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `List[hop.render.Label]`.
pub fn list_label(v: &[crate::hop::render::Label]) -> R<bool> {
    for x in v {
        if !label(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `hop.world.View`.
pub fn view(v: &crate::hop::world::View) -> R<bool> {
    if !level(&v.stage)? {
        return Ok(false);
    }
    if !player(&v.player)? {
        return Ok(false);
    }
    list_crawler(&v.crawlers)
}

/// Whether a value is of `hop.render.Frame`.
pub fn frame(v: &crate::hop::render::Frame) -> R<bool> {
    if !rgb(&v.sky)? {
        return Ok(false);
    }
    if !vec3(&v.eye)? {
        return Ok(false);
    }
    if !vec3(&v.target)? {
        return Ok(false);
    }
    if !list_draw(&v.world)? {
        return Ok(false);
    }
    list_label(&v.labels)
}

/// Whether a value is of `hop.render.Draw`.
pub fn draw(v: &crate::hop::render::Draw) -> R<bool> {
    match v {
        crate::hop::render::Draw::Cube { r#box: x0, color: x1, .. } => {
            if !r#box(x0)? {
                return Ok(false);
            }
            if !rgb(x1)? {
                return Ok(false);
            }
        }
        crate::hop::render::Draw::Wires { r#box: x0, color: x1, .. } => {
            if !r#box(x0)? {
                return Ok(false);
            }
            if !rgb(x1)? {
                return Ok(false);
            }
        }
        crate::hop::render::Draw::Mesh { at: x1, yaw: x2, .. } => {
            if !vec3(x1)? {
                return Ok(false);
            }
            if !range_0_359(x2)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Whether a value is of `1..=10000`.
pub fn range_1_10000(v: &Int) -> R<bool> {
    Ok((1..=10000).contains(v))
}

/// Whether a value is of `hop.model.Player`.
pub fn player(v: &crate::hop::model::Player) -> R<bool> {
    if !vec3(&v.at)? {
        return Ok(false);
    }
    if !vec3(&v.vel)? {
        return Ok(false);
    }
    if !range_0_6(&v.coyote)? {
        return Ok(false);
    }
    range_0_359(&v.yaw)
}

/// Whether a value is of `Map[Int, hop.model.Crawler]`.
pub fn map_int_crawler(v: &BTreeMap<Int, crate::hop::model::Crawler>) -> R<bool> {
    for x in v.values() {
        if !crawler(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `0..=3`.
pub fn range_0_3(v: &Int) -> R<bool> {
    Ok((0..=3).contains(v))
}

/// Whether a value is of `1..=2`.
pub fn range_1_2(v: &Int) -> R<bool> {
    Ok((1..=2).contains(v))
}

/// Whether a value is of `hop.model.Gem`.
pub fn gem(v: &crate::hop::model::Gem) -> R<bool> {
    vec3(&v.at)
}

/// Whether a value is of `-1048576..=1048576`.
pub fn range_minus1048576_1048576(v: &Int) -> R<bool> {
    Ok((-1048576..=1048576).contains(v))
}

/// Whether a value is of `hop.physics.Placed`.
pub fn placed(v: &crate::hop::physics::Placed) -> R<bool> {
    r#box(&v.r#box)
}

/// Whether a value is of `hop.render.Label`.
pub fn label(v: &crate::hop::render::Label) -> R<bool> {
    rgb(&v.color)
}
