//! Ground: terrain heightfields, slopes, and what a body stands on (Vol. III Ch. 1 §1.10–1.11;
//! Amendment A-4).
//!
//! A region may declare terrain: a grid of height samples in its own frame, sample
//! `[column, row]` standing at `(column × spacing, row × spacing)` ([`TERRAIN_SPACING`],
//! [`TERRAIN_SAMPLE`]). Ground between samples is the bilinear blend of the four around it —
//! continuous, so a body walking downhill descends smoothly, and exact in integers. Where a
//! region has no terrain (a room, a deck) or the point lies outside its samples, the ground is
//! the frame's level floor, `z = 0`.
//!
//! A body's **support** is the highest of: the ground beneath its base, and the top of any solid
//! body in the same container whose footprint it stands over, provided that top is within a
//! step of the body's base (a table can be stood on; a wall's top cannot be stepped onto from the
//! floor). Gravity and travel both ask this one question, so they can never disagree about
//! whether a body is standing on something.

use crate::index::{container_of, size_of};
use crate::schema::{SOLID, TERRAIN_SAMPLE, TERRAIN_SPACING};
use crate::shape::body_box;
use kernel::fact::FactKey;
use kernel::fixed::{div_round, isqrt};
use kernel::identity::EntityId;
use kernel::spatial::Aabb;
use kernel::system::CommittedView;
use kernel::value::Value;

/// A region's terrain grid spacing in centimetres, if it has terrain.
pub fn spacing(view: &dyn CommittedView, region: EntityId) -> Option<i64> {
    view.read(FactKey::new(region, TERRAIN_SPACING))?
        .value
        .as_int()
        .filter(|s| *s > 0)
}

/// The height of terrain sample `[column, row]` in `region`, if the region has one there. Reads
/// just that sample (a range read of the set), never the whole heightfield.
pub fn sample(view: &dyn CommittedView, region: EntityId, column: i64, row: i64) -> Option<i64> {
    let lo = Value::Vec3([column, row, i64::MIN]);
    let hi = Value::Vec3([column, row, i64::MAX]);
    view.read_range(FactKey::new(region, TERRAIN_SAMPLE), &lo, &hi)
        .first()
        .and_then(|f| f.value.as_vec3())
        .map(|v| v[2])
}

/// The four samples of the grid cell containing `(x, y)`, with the point's offsets into it —
/// `None` if the region has no terrain or any sample the point needs is missing. A point on a
/// cell's edge needs only the samples along that edge.
fn cell(
    view: &dyn CommittedView,
    region: EntityId,
    x: i64,
    y: i64,
) -> Option<(i64, [i64; 4], i64, i64)> {
    let sp = spacing(view, region)?;
    let (c, r) = (x.div_euclid(sp), y.div_euclid(sp));
    let (fx, fy) = (x.rem_euclid(sp), y.rem_euclid(sp));
    let h00 = sample(view, region, c, r)?;
    let h10 = if fx == 0 {
        h00
    } else {
        sample(view, region, c + 1, r)?
    };
    let h01 = if fy == 0 {
        h00
    } else {
        sample(view, region, c, r + 1)?
    };
    let h11 = match (fx == 0, fy == 0) {
        (true, true) => h00,
        (true, false) => h01,
        (false, true) => h10,
        (false, false) => sample(view, region, c + 1, r + 1)?,
    };
    Some((sp, [h00, h10, h01, h11], fx, fy))
}

/// The terrain height at `(x, y)` in `region`'s frame, if its terrain covers that point: the
/// bilinear blend of the cell's four samples, rounded to the centimetre.
pub fn terrain_height(view: &dyn CommittedView, region: EntityId, x: i64, y: i64) -> Option<i64> {
    let (sp, [h00, h10, h01, h11], fx, fy) = cell(view, region, x, y)?;
    let (sp, fx, fy) = (sp as i128, fx as i128, fy as i128);
    let blend = (h00 as i128 * (sp - fx) + h10 as i128 * fx) * (sp - fy)
        + (h01 as i128 * (sp - fx) + h11 as i128 * fx) * fy;
    Some(div_round(blend, sp * sp) as i64)
}

/// The ground at `(x, y)` in `region`'s frame: its terrain where it has some, else its level
/// floor at 0.
pub fn ground(view: &dyn CommittedView, region: EntityId, x: i64, y: i64) -> i64 {
    terrain_height(view, region, x, y).unwrap_or(0)
}

/// The steepness of the terrain cell containing `(x, y)`, as a percentage grade (rise over run
/// × 100: 100 is 45°), or `None` where the region has no terrain. Uses the cell's average
/// gradient — one slope per cell, so a path is judged the same wherever in the cell it crosses.
pub fn slope_percent(view: &dyn CommittedView, region: EntityId, x: i64, y: i64) -> Option<i64> {
    let sp = spacing(view, region)?;
    let (c, r) = (x.div_euclid(sp), y.div_euclid(sp));
    let corner = |dc, dr| sample(view, region, c + dc, r + dr);
    let (h00, h10, h01, h11) = (corner(0, 0)?, corner(1, 0)?, corner(0, 1)?, corner(1, 1)?);
    // Rise across the cell in x and in y, each averaged over the cell's two edges; the grade is
    // their magnitude over the run (two spacings, since each rise is summed over two edges).
    let gx = (h10 - h00 + h11 - h01) as i128;
    let gy = (h01 - h00 + h11 - h10) as i128;
    let rise = isqrt((gx * gx + gy * gy) as u128) as i128;
    Some(div_round(rise * 100, 2 * sp as i128) as i64)
}

/// What `body` is standing on, as a height in its container's frame: the ground under its base,
/// or the top of a solid body (not itself) whose footprint it stands over and whose top is no
/// more than `step` above its base — whichever is higher. A body with no container has nothing
/// to stand on and reports its own height (it is supported by definition).
pub fn support(view: &dyn CommittedView, body: EntityId, step: i64) -> i64 {
    let Some(frame) = container_of(view, body) else {
        return crate::space::local_position(view, body)[2];
    };
    let at = crate::space::local_position(view, body);
    support_at(view, frame, at, step, &[body])
}

/// The support at point `at` in `frame`, for something whose base is at `at[2]` and which can
/// step up `step`: the ground there, or the highest reachable solid top beneath it. Bodies in
/// `ignore` are not stood on (the body itself; a travel target).
pub fn support_at(
    view: &dyn CommittedView,
    frame: EntityId,
    at: [i64; 3],
    step: i64,
    ignore: &[EntityId],
) -> i64 {
    let mut best = ground(view, frame, at[0], at[1]);
    let reach = at[2].saturating_add(step.max(0));
    // Only a solid whose top lies between the ground and the reach can matter: the column
    // searched is exactly that span, which keeps the index query small.
    for solid in solids_under(view, frame, at, best, reach) {
        if ignore.contains(&solid) {
            continue;
        }
        if let Some(b) = body_box(view, solid) {
            if b.footprint_contains(at, 0) && b.top() <= reach && b.top() > best {
                best = b.top();
            }
        }
    }
    best
}

/// The solid bodies placed directly in `frame` whose boxes meet the vertical column through `at`
/// between heights `from` and `up_to` — through the spatial index when one is installed, by
/// scanning otherwise.
fn solids_under(
    view: &dyn CommittedView,
    frame: EntityId,
    at: [i64; 3],
    from: i64,
    up_to: i64,
) -> Vec<EntityId> {
    let column = Aabb::new([at[0], at[1], from], [at[0], at[1], up_to.max(from)]);
    let candidates: Vec<EntityId> = match view.spatial() {
        Some(index) => index
            .candidates(frame, &column)
            .into_iter()
            .map(|(e, _)| e)
            .collect(),
        None => crate::nearby::contents(view, frame),
    };
    candidates
        .into_iter()
        .filter(|e| is_true(view, *e, SOLID) && size_of(view, *e).is_some())
        .collect()
}

/// Whether boolean fact `fact` is set true on `entity` (absent is false).
pub fn is_true(view: &dyn CommittedView, entity: EntityId, fact: kernel::fact::FactType) -> bool {
    matches!(
        view.read(FactKey::new(entity, fact)).map(|f| f.value),
        Some(Value::Bool(true))
    )
}
