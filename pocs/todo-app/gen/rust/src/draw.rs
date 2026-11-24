//! A generator per declared type a property, law or trace draws from: the engine's draws.

use crate::rt::{self, Int, Opt, R};
use crate::rt::rng::Rng;
use crate::machine::{Machine, Pools};
use std::collections::{BTreeMap, BTreeSet};

/// A value of `todo.model.Title`.
pub fn title(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<String> {
    for _ in 0..rt::rng::TRIES {
        let v = text_where0(m, rng, pools)?;
        if crate::conform::title(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `todo.model.Title` found in 400 tries"))
}

/// A value of `todo.model.Priority`.
pub fn priority(
    _m: &mut Machine,
    rng: &mut Rng,
    _pools: &Pools,
) -> R<crate::todo::model::Priority> {
    Ok(
        match rng.below(3) {
            0 => crate::todo::model::Priority::Low,
            1 => crate::todo::model::Priority::Normal,
            _ => crate::todo::model::Priority::High,
        },
    )
}

/// A value of `Set[todo.model.Tag]`.
pub fn set_tag(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<BTreeSet<String>> {
    for _ in 0..rt::rng::TRIES {
        let v = {
            let n = rng.below(4);
            let mut xs = BTreeSet::new();
            for _ in 0..n {
                xs.insert(tag(m, rng, pools)?);
            }
            xs
        };
        if crate::conform::set_tag(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `Set[todo.model.Tag]` found in 400 tries"))
}

/// A value of `Bool`.
pub fn bool(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<bool> {
    Ok(rng.chance(1, 2))
}

/// A value of `Option[todo.model.Tag]`.
pub fn option_tag(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<Opt<String>> {
    for _ in 0..rt::rng::TRIES {
        let v = if rng.chance(1, 3) {
            Opt::None
        } else {
            Opt::Some(tag(m, rng, pools)?)
        };
        if crate::conform::option_tag(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `Option[todo.model.Tag]` found in 400 tries"))
}

/// A value of `todo.model.TaskNo`.
pub fn task_no(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = match rt::rng::from_pool(rng, &pools.p0) {
            Some(v) => v,
            None => int_where2(m, rng, pools)?,
        };
        if crate::conform::task_no(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `todo.model.TaskNo` found in 400 tries"))
}

/// A value of `Map[todo.model.TaskNo, todo.model.Task]`.
pub fn map_task_no_task(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<BTreeMap<Int, crate::todo::model::Task>> {
    for _ in 0..rt::rng::TRIES {
        let v = {
            let n = rng.below(4);
            let mut map = BTreeMap::new();
            for _ in 0..n {
                let k = task_no(m, rng, pools)?;
                let v = task(m, rng, pools)?;
                map.insert(k, v);
            }
            map
        };
        if crate::conform::map_task_no_task(&v)? {
            return Ok(v);
        }
    }
    Err(
        rt::stop(
            "E-GEN",
            "no value of `Map[todo.model.TaskNo, todo.model.Task]` found in 400 tries",
        ),
    )
}

/// A value of `Text where is_title(self)`.
pub fn text_where0(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<String> {
    for _ in 0..rt::rng::TRIES {
        let v = text(m, rng, pools)?;
        if crate::conform::text_where0(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `Text where is_title(self)` found in 400 tries"))
}

/// A value of `todo.model.Tag`.
pub fn tag(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<String> {
    for _ in 0..rt::rng::TRIES {
        let v = text_where1(m, rng, pools)?;
        if crate::conform::tag(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `todo.model.Tag` found in 400 tries"))
}

/// A value of `Int where self >= 1`.
pub fn int_where2(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<Int> {
    for _ in 0..rt::rng::TRIES {
        let v = int(m, rng, pools)?;
        if crate::conform::int_where2(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `Int where self >= 1` found in 400 tries"))
}

/// A value of `todo.model.Task`.
pub fn task(
    m: &mut Machine,
    rng: &mut Rng,
    pools: &Pools,
) -> R<crate::todo::model::Task> {
    for _ in 0..rt::rng::TRIES {
        let v = crate::todo::model::Task {
            no: task_no(m, rng, pools)?,
            title: title(m, rng, pools)?,
            status: status(m, rng, pools)?,
            priority: priority(m, rng, pools)?,
            tags: set_tag(m, rng, pools)?,
        };
        if crate::conform::task(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `todo.model.Task` found in 400 tries"))
}

/// A value of `Text`.
pub fn text(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<String> {
    Ok(rt::rng::text(rng))
}

/// A value of `Text where is_tag(self)`.
pub fn text_where1(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<String> {
    for _ in 0..rt::rng::TRIES {
        let v = text(m, rng, pools)?;
        if crate::conform::text_where1(&v)? {
            return Ok(v);
        }
    }
    Err(rt::stop("E-GEN", "no value of `Text where is_tag(self)` found in 400 tries"))
}

/// A value of `Int`.
pub fn int(_m: &mut Machine, rng: &mut Rng, _pools: &Pools) -> R<Int> {
    Ok(rt::rng::int(rng))
}

/// A value of `todo.model.Status`.
pub fn status(
    _m: &mut Machine,
    rng: &mut Rng,
    _pools: &Pools,
) -> R<crate::todo::model::Status> {
    Ok(
        match rng.below(2) {
            0 => crate::todo::model::Status::Open,
            _ => crate::todo::model::Status::Done,
        },
    )
}
