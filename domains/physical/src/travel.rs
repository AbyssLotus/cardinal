//! Gravity and travel: how bodies move under the world's constraints (Vol. III Ch. 1 §1.11;
//! Appendix A, Rulings 4 and 13; Amendment A-4).
//!
//! **Who decides, who moves.** A decider — a player's validated action, an NPC's choice —
//! proposes where a body should go ([`TRAVEL_TO`]) and how fast ([`TRAVEL_SPEED`]). Physical
//! Reality alone moves it (Ruling 13): [`Travel`] finds the way and lays down motion segments,
//! one straight leg at a time; [`Gravity`] drops whatever nothing holds up. Neither ever decides
//! *whether* to go anywhere.
//!
//! **What Travel honours.**
//! - Between regions, it routes only through portals that are **open**, that the body **fits**
//!   (no wider and no taller than the opening; an opening without a size constrains nothing),
//!   and that it can **reach** (no higher above the floor than the body is tall), and passes
//!   through an opening to its **far side** — the linked face, or else the nearest face back. If no such way exists it reports the travel **blocked**
//!   ([`TRAVEL_BLOCKED`]), and keeps the intent: open the door and it goes on.
//! - Within a region, it walks straight when nothing **solid** and taller than a step stands in
//!   the way and the ground is not too steep; otherwise it plans around obstacles on a grid
//!   (A*, eight-connected, never cutting a corner) and walks to the farthest point of that path
//!   it can reach in a straight line. Waypoints sit on the ground (or on a low solid top), so a
//!   walker follows terrain from point to point.
//! - **One body per opening per tick**: if two would pass through the same opening on the same
//!   tick, the lower id goes and the other waits a tick.
//! - It acts only on a body that is standing on something and not mid-leg; a body that is
//!   falling is gravity's, and a body between waypoints keeps walking.
//!
//! **What Gravity does.** A [`MOBILE`] body that is not moving and stands higher than its
//! support (the ground, or a solid top within a step) starts a fall straight down to that
//! support, taking the time gravity takes (`√(2h/g)`, rounded up to whole ticks), and records how
//! far it fell ([`FALL_HEIGHT`]) for anyone who cares what a fall does to a body. The fall is a
//! motion segment like any other — kinematic, not a rigid-body simulation.
//!
//! The two systems never act on the same body in the same tick: Travel acts only on supported
//! bodies and Gravity only on unsupported ones, judged by the same [`support`] on the same view.

use crate::index::{container_of, size_of};
use crate::motion::{compass, depart, halt, is_moving, segment};
use crate::schema::{
    BODY_SIZE, CONTAINED_IN, FALL_HEIGHT, HAS_PORTAL, HEADING, LEADS_TO, MOBILE, MOTION_END,
    MOTION_START, MOTION_TARGET, PORTAL_FAR_SIDE, PORTAL_OPEN, POSITION, SOLID, TERRAIN_SAMPLE,
    TERRAIN_SPACING, TRAVEL_BLOCKED, TRAVEL_SPEED, TRAVEL_TO,
};
use crate::shape::{body_box, BodyBox};
use crate::space::{local_position, portal_destination, portals_in, route_where};
use crate::terrain::{ground, is_true, slope_percent, support, support_at};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::isqrt;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::SimClock;
use kernel::value::Value;
use std::collections::BTreeSet;

/// How close counts as "there", in centimetres, beyond the bodies' own sizes. A representation
/// tolerance — the gap below which two placements are the same place — not a world rule.
const ARRIVAL_TOLERANCE: i64 = 5;

/// How far above its support a body may hover before it counts as unsupported, in centimetres:
/// the same kind of tolerance, absorbing the centimetre that rounding can leave.
const SUPPORT_TOLERANCE: i64 = 1;

/// The most cells a planning grid may have along either side; larger areas use coarser cells,
/// so planning cost is bounded whatever the region's size.
const MAX_GRID: i64 = 256;

/// The movement rules both systems share, all world-package data (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MoveRules {
    /// The world's clock.
    pub clock: SimClock,
    /// Gravitational acceleration, in centimetres per second squared.
    pub gravity_cm_s2: i64,
    /// The highest a body steps up without climbing, in centimetres.
    pub step_height_cm: i64,
    /// The steepest ground a body walks over, as a percentage grade.
    pub max_slope_percent: i64,
    /// The cell size of a travel planning grid, in centimetres.
    pub nav_cell_cm: i64,
}

/// A body's horizontal radius: the larger of its half-width and half-depth (0 for a point).
fn radius(view: &dyn CommittedView, e: EntityId) -> i64 {
    size_of(view, e).map_or(0, |s| s[0].max(s[1]))
}

/// Horizontal distance between two points.
fn flat_distance(a: [i64; 3], b: [i64; 3]) -> i64 {
    let dx = (a[0] - b[0]) as i128;
    let dy = (a[1] - b[1]) as i128;
    isqrt((dx * dx + dy * dy) as u128) as i64
}

/// Whether `body` is standing on its support (within the tolerance) or below it.
pub fn supported(view: &dyn CommittedView, body: EntityId, step: i64) -> bool {
    let z = local_position(view, body)[2];
    z - support(view, body, step) <= SUPPORT_TOLERANCE
}

/// Whether `portal` is open (absent means open).
pub fn is_open(view: &dyn CommittedView, portal: EntityId) -> bool {
    !matches!(
        view.read(FactKey::new(portal, PORTAL_OPEN))
            .map(|f| f.value),
        Some(Value::Bool(false))
    )
}

/// Whether `body` fits through `portal`: no wider and no taller than the opening. An opening or
/// a body without a size constrains nothing.
pub fn fits(view: &dyn CommittedView, body: EntityId, portal: EntityId) -> bool {
    match (size_of(view, body), size_of(view, portal)) {
        (Some(b), Some(p)) => b[0] <= p[0] && b[2] <= p[2],
        _ => true,
    }
}

/// Whether `body` can get up into `portal` from the floor beneath it: the opening's base stands
/// no higher above that floor than the body is tall (or a step, for something shorter than a
/// step). A cat can hop onto a low sill; nobody walks into a bedroom window from the yard.
pub fn reachable(view: &dyn CommittedView, body: EntityId, portal: EntityId, step: i64) -> bool {
    let Some(host) = container_of(view, portal) else {
        return false;
    };
    let at = local_position(view, portal);
    let climb = size_of(view, body).map_or(0, |s| s[2]).max(step);
    at[2] - ground(view, host, at[0], at[1]) <= climb
}

/// The face something emerges from after passing through `portal` into `dest`: the linked far
/// side if the world declared one, else the destination's portal leading back to `portal`'s host
/// that is nearest to `portal` (lowest id on a tie), else none.
pub fn far_side(view: &dyn CommittedView, portal: EntityId, dest: EntityId) -> Option<EntityId> {
    if let Some(Value::Entity(linked)) = view
        .read(FactKey::new(portal, PORTAL_FAR_SIDE))
        .map(|f| f.value)
    {
        return Some(linked);
    }
    let host = container_of(view, portal)?;
    portals_in(view, dest)
        .into_iter()
        .filter(|q| portal_destination(view, *q) == Some(host))
        .min_by_key(|q| {
            (
                crate::space::distance(view, portal, *q).unwrap_or(i64::MAX),
                *q,
            )
        })
}

/// Whether `e` is a place one can be *in* — a region with openings, or one an opening leads to —
/// rather than a thing one goes up to.
fn is_place(view: &dyn CommittedView, e: EntityId) -> bool {
    !view.read_all(FactKey::new(e, HAS_PORTAL)).is_empty()
        || view
            .entities_with(LEADS_TO)
            .into_iter()
            .any(|p| portal_destination(view, p) == Some(e))
}

/// Solid bodies in `frame` too tall to step onto from the ground — the obstacles a walker goes
/// around — with their boxes. `ignore` lists bodies that are not obstacles (the walker; what it
/// is walking up to). Whatever the walker is standing *on* is not an obstacle either: someone on
/// a shed's roof walks across it, and off its edge.
fn obstacles(
    view: &dyn CommittedView,
    frame: EntityId,
    step: i64,
    walker: EntityId,
    ignore: &[EntityId],
) -> Vec<BodyBox> {
    let at = local_position(view, walker);
    crate::nearby::contents(view, frame)
        .into_iter()
        .filter(|e| *e != walker && !ignore.contains(e) && is_true(view, *e, SOLID))
        .filter_map(|e| body_box(view, e))
        .filter(|b| b.top() > ground(view, frame, b.base[0], b.base[1]) + step)
        .filter(|b| !(b.footprint_contains(at, 0) && b.top() <= at[2] + step))
        .collect()
}

/// Whether a walker of horizontal radius `r` can go straight from `a` to `b` in `frame`: no
/// obstacle's footprint (grown by `r`) crosses the track, and no ground along it is steeper than
/// the rules allow.
fn clear(
    view: &dyn CommittedView,
    frame: EntityId,
    rules: &MoveRules,
    walls: &[BodyBox],
    r: i64,
    a: [i64; 3],
    b: [i64; 3],
) -> bool {
    if walls.iter().any(|w| w.track_hits(a, b, r)) {
        return false;
    }
    let run = flat_distance(a, b);
    let samples = (run / rules.nav_cell_cm.max(1)).clamp(1, 4096);
    (0..=samples).all(|k| {
        let x = a[0] + (b[0] - a[0]) * k / samples;
        let y = a[1] + (b[1] - a[1]) * k / samples;
        slope_percent(view, frame, x, y).map_or(true, |s| s <= rules.max_slope_percent)
    })
}

/// The next point a walker of radius `r` standing at `start` in `frame` should walk straight to,
/// on its way to `goal` (stopping `stop` short of it): the goal itself if the way is clear, else
/// the farthest clear point along a planned path around the obstacles. `None` if no path exists.
#[allow(clippy::too_many_arguments)]
fn next_waypoint(
    view: &dyn CommittedView,
    rules: &MoveRules,
    frame: EntityId,
    walls: &[BodyBox],
    r: i64,
    start: [i64; 3],
    goal: [i64; 3],
    stop: i64,
) -> Option<[i64; 2]> {
    // The goal, shortened by `stop` along the straight line to it.
    let len = flat_distance(start, goal);
    let end = if stop > 0 && len > stop {
        let keep = (len - stop) as i128;
        [
            start[0] + ((goal[0] - start[0]) as i128 * keep / len as i128) as i64,
            start[1] + ((goal[1] - start[1]) as i128 * keep / len as i128) as i64,
        ]
    } else {
        [goal[0], goal[1]]
    };
    let end3 = [end[0], end[1], start[2]];
    if clear(view, frame, rules, walls, r, start, end3) {
        return Some(end);
    }

    // Plan on a grid over the region — its size, if it has one; else the ground its terrain
    // covers, if it has terrain; else the area around the walker, the goal, and the obstacles —
    // coarsened if needed so neither side exceeds MAX_GRID cells.
    let (lo, hi) = match (size_of(view, frame), terrain_extent(view, frame)) {
        (Some(s), _) => ([-s[0], -s[1]], [s[0], s[1]]),
        (None, Some(far)) => (
            [start[0].min(goal[0]).min(0), start[1].min(goal[1]).min(0)],
            [
                start[0].max(goal[0]).max(far[0]),
                start[1].max(goal[1]).max(far[1]),
            ],
        ),
        (None, None) => {
            let mut lo = [start[0].min(goal[0]), start[1].min(goal[1])];
            let mut hi = [start[0].max(goal[0]), start[1].max(goal[1])];
            for w in walls {
                let reach = w.size[0].max(w.size[1]) + r;
                lo = [lo[0].min(w.base[0] - reach), lo[1].min(w.base[1] - reach)];
                hi = [hi[0].max(w.base[0] + reach), hi[1].max(w.base[1] + reach)];
            }
            let pad = 4 * rules.nav_cell_cm.max(1);
            ([lo[0] - pad, lo[1] - pad], [hi[0] + pad, hi[1] + pad])
        }
    };
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1);
    let cell = rules
        .nav_cell_cm
        .max(1)
        .max((span + MAX_GRID - 1) / MAX_GRID);
    let (nx, ny) = ((hi[0] - lo[0]) / cell + 1, (hi[1] - lo[1]) / cell + 1);
    let centre = |i: i64, j: i64| {
        [
            lo[0] + i * cell + cell / 2,
            lo[1] + j * cell + cell / 2,
            start[2],
        ]
    };
    let cell_of = |p: [i64; 3]| {
        (
            ((p[0] - lo[0]) / cell).clamp(0, nx - 1),
            ((p[1] - lo[1]) / cell).clamp(0, ny - 1),
        )
    };
    let (si, sj) = cell_of(start);
    let (gi, gj) = cell_of(goal);
    let blocked = |i: i64, j: i64| {
        if (i, j) == (si, sj) || (i, j) == (gi, gj) {
            return false;
        }
        let c = centre(i, j);
        walls.iter().any(|w| w.footprint_contains(c, r))
            || slope_percent(view, frame, c[0], c[1]).is_some_and(|s| s > rules.max_slope_percent)
    };
    let path = astar(nx, ny, (si, sj), (gi, gj), blocked)?;

    // Walk to the farthest point of the path reachable in a straight line.
    for &(i, j) in path.iter().rev() {
        let target = if (i, j) == (gi, gj) {
            end3
        } else {
            centre(i, j)
        };
        if clear(view, frame, rules, walls, r, start, target) {
            return Some([target[0], target[1]]);
        }
    }
    // The first step of the path is always adjacent and clear of corners.
    path.get(1).map(|&(i, j)| {
        let c = centre(i, j);
        [c[0], c[1]]
    })
}

/// The far corner of `frame`'s terrain grid (its last column and row, in centimetres), if it has
/// terrain: a region with terrain and no declared size is the ground its terrain covers.
fn terrain_extent(view: &dyn CommittedView, frame: EntityId) -> Option<[i64; 2]> {
    let sp = crate::terrain::spacing(view, frame)?;
    let samples = view.read_all(FactKey::new(frame, TERRAIN_SAMPLE));
    let (mut cols, mut rows) = (0i64, 0i64);
    for f in samples {
        if let Some([c, r, _]) = f.value.as_vec3() {
            cols = cols.max(c);
            rows = rows.max(r);
        }
    }
    Some([cols * sp, rows * sp])
}

/// Deterministic A* over an `nx × ny` grid, eight-connected, never cutting a blocked corner:
/// straight steps cost 10, diagonal 14, under the octile heuristic. Ties are broken by the
/// remaining estimate and then by cell coordinates, so the same grid always yields the same
/// path. Returns the path from `start` to `goal` inclusive.
fn astar(
    nx: i64,
    ny: i64,
    start: (i64, i64),
    goal: (i64, i64),
    blocked: impl Fn(i64, i64) -> bool,
) -> Option<Vec<(i64, i64)>> {
    let idx = |(i, j): (i64, i64)| (j * nx + i) as usize;
    let h = |(i, j): (i64, i64)| {
        let (dx, dy) = ((i - goal.0).abs(), (j - goal.1).abs());
        10 * dx.max(dy) + 4 * dx.min(dy)
    };
    let n = (nx * ny) as usize;
    let mut g = vec![i64::MAX; n];
    let mut from: Vec<Option<(i64, i64)>> = vec![None; n];
    let mut closed = vec![false; n];
    let mut open = BTreeSet::new();
    g[idx(start)] = 0;
    open.insert((h(start), h(start), start.1, start.0));
    while let Some((_, _, j, i)) = open.pop_first() {
        let here = (i, j);
        if closed[idx(here)] {
            continue;
        }
        closed[idx(here)] = true;
        if here == goal {
            let mut path = vec![here];
            let mut at = here;
            while let Some(prev) = from[idx(at)] {
                path.push(prev);
                at = prev;
            }
            path.reverse();
            return Some(path);
        }
        for (di, dj) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let next = (i + di, j + dj);
            if next.0 < 0 || next.1 < 0 || next.0 >= nx || next.1 >= ny || blocked(next.0, next.1) {
                continue;
            }
            let diagonal = di != 0 && dj != 0;
            if diagonal && (blocked(i + di, j) || blocked(i, j + dj)) {
                continue;
            }
            let cost = g[idx(here)] + if diagonal { 14 } else { 10 };
            if cost < g[idx(next)] {
                g[idx(next)] = cost;
                from[idx(next)] = Some(here);
                open.insert((cost + h(next), h(next), next.1, next.0));
            }
        }
    }
    None
}

const GRAVITY_READS: &[FactType] = &[
    MOBILE,
    SOLID,
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    TERRAIN_SPACING,
    TERRAIN_SAMPLE,
];
const GRAVITY_WRITES: &[FactType] = &[
    POSITION,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    FALL_HEIGHT,
];

/// Drops every unsupported mobile body to whatever is under it (Amendment A-4).
pub struct Gravity {
    rules: MoveRules,
}

impl Gravity {
    /// Gravity under the world's movement rules.
    pub const fn new(rules: MoveRules) -> Self {
        Self { rules }
    }
}

impl System for Gravity {
    fn id(&self) -> SystemId {
        SystemId::new("physical.gravity")
    }
    fn reads(&self) -> &'static [FactType] {
        GRAVITY_READS
    }
    fn writes(&self) -> &'static [FactType] {
        GRAVITY_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for body in view.entities_with(MOBILE) {
            if !is_true(view, body, MOBILE) || container_of(view, body).is_none() {
                continue;
            }
            if is_moving(view, body) || supported(view, body, self.rules.step_height_cm) {
                continue;
            }
            let here = local_position(view, body);
            let floor = support(view, body, self.rules.step_height_cm);
            let drop = here[2] - floor;
            // Time to fall h from rest: √(2h/g) seconds = √(2h·10⁶/g) milliseconds.
            let g = self.rules.gravity_cm_s2.max(1) as u128;
            let fall_ms = isqrt(2 * drop as u128 * 1_000_000 / g) as u64;
            let tick_ms = self.rules.clock.tick_ms();
            let ticks = fall_ms.div_ceil(tick_ms).max(1);
            let now = view.tick() as i64;
            let landing = [here[0], here[1], floor];
            let cause = Cause::new("fall");
            let mut push = |key: FactKey, change: Change| {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    change,
                    cause,
                ));
            };
            push(FactKey::new(body, POSITION), Change::Set(Value::Vec3(here)));
            push(
                FactKey::new(body, MOTION_TARGET),
                Change::Set(Value::Vec3(landing)),
            );
            push(
                FactKey::new(body, MOTION_START),
                Change::Set(Value::Int(now)),
            );
            push(
                FactKey::new(body, MOTION_END),
                Change::Set(Value::Int(now + ticks as i64)),
            );
            push(
                FactKey::new(body, FALL_HEIGHT),
                Change::Set(Value::Int(drop)),
            );
        }
        out
    }
}

const TRAVEL_READS: &[FactType] = &[
    TRAVEL_TO,
    TRAVEL_SPEED,
    TRAVEL_BLOCKED,
    MOBILE,
    SOLID,
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    HAS_PORTAL,
    LEADS_TO,
    PORTAL_OPEN,
    PORTAL_FAR_SIDE,
    TERRAIN_SPACING,
    TERRAIN_SAMPLE,
];
const TRAVEL_WRITES: &[FactType] = &[
    TRAVEL_TO,
    TRAVEL_BLOCKED,
    CONTAINED_IN,
    POSITION,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
];

/// Carries out travel intents (Ruling 13): routes, walks, passes through openings, arrives, or
/// reports the way blocked.
pub struct Travel {
    rules: MoveRules,
}

impl Travel {
    /// Travel under the world's movement rules.
    pub const fn new(rules: MoveRules) -> Self {
        Self { rules }
    }
}

/// What a traveller does this tick.
enum Step {
    /// Nothing yet (mid-leg, falling, waiting its turn at an opening, no speed given).
    Wait,
    /// It is where it was asked to be.
    Arrive,
    /// No open way it fits through leads there.
    Blocked,
    /// Walk straight to this point in its current region.
    Walk([i64; 2]),
    /// Pass through this portal now.
    Pass(EntityId),
}

impl Travel {
    /// Decide one traveller's step. `taken` holds the openings already being passed this tick.
    fn step_for(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
        taken: &BTreeSet<EntityId>,
    ) -> Step {
        let Some(Value::Entity(target)) = view.read(FactKey::new(body, TRAVEL_TO)).map(|f| f.value)
        else {
            return Step::Wait;
        };
        let speed = view
            .read(FactKey::new(body, TRAVEL_SPEED))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0);
        let step = self.rules.step_height_cm;
        if speed <= 0 || is_moving(view, body) || !supported(view, body, step) {
            return Step::Wait;
        }
        let Some(here) = container_of(view, body) else {
            return Step::Wait;
        };
        let at = local_position(view, body);

        // Where must it be? Inside a place, or beside a thing.
        let (goal_region, approach) = if is_place(view, target) {
            (target, None)
        } else {
            let Some(region) = container_of(view, target) else {
                return Step::Blocked;
            };
            (region, Some(target))
        };
        if here == goal_region {
            let Some(thing) = approach else {
                return Step::Arrive;
            };
            let goal = local_position(view, thing);
            let reach = radius(view, body) + radius(view, thing) + ARRIVAL_TOLERANCE;
            if flat_distance(at, goal) <= reach {
                return Step::Arrive;
            }
            let walls = obstacles(view, here, step, body, &[thing]);
            return match next_waypoint(
                view,
                &self.rules,
                here,
                &walls,
                radius(view, body),
                at,
                goal,
                reach - ARRIVAL_TOLERANCE,
            ) {
                Some(w) => Step::Walk(w),
                None => Step::Blocked,
            };
        }

        // Another region: the way out, through openings it fits that are open.
        let usable =
            |p: EntityId| is_open(view, p) && fits(view, body, p) && reachable(view, body, p, step);
        let Some(route) = route_where(view, here, goal_region, usable) else {
            return Step::Blocked;
        };
        let portal = route[0];
        let door = local_position(view, portal);
        if flat_distance(at, door) <= ARRIVAL_TOLERANCE {
            let dest = portal_destination(view, portal);
            let partner = dest.and_then(|d| far_side(view, portal, d));
            if taken.contains(&portal) || partner.is_some_and(|q| taken.contains(&q)) {
                return Step::Wait;
            }
            return Step::Pass(portal);
        }
        let walls = obstacles(view, here, step, body, &[]);
        match next_waypoint(
            view,
            &self.rules,
            here,
            &walls,
            radius(view, body),
            at,
            door,
            0,
        ) {
            Some(w) => Step::Walk(w),
            None => Step::Blocked,
        }
    }
}

impl System for Travel {
    fn id(&self) -> SystemId {
        SystemId::new("physical.travel")
    }
    fn reads(&self) -> &'static [FactType] {
        TRAVEL_READS
    }
    fn writes(&self) -> &'static [FactType] {
        TRAVEL_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        let mut taken: BTreeSet<EntityId> = BTreeSet::new();
        let was_blocked = |b: EntityId| is_true(view, b, TRAVEL_BLOCKED);
        // Ascending id: when two bodies reach one opening on one tick, the lower id goes first.
        for body in view.entities_with(TRAVEL_TO) {
            let mut changes: Vec<(FactKey, Change, &'static str)> = Vec::new();
            let step = self.step_for(view, body, &taken);
            if !matches!(step, Step::Blocked | Step::Wait) && was_blocked(body) {
                changes.push((
                    FactKey::new(body, TRAVEL_BLOCKED),
                    Change::Tombstone,
                    "way_open",
                ));
            }
            match step {
                Step::Wait => {}
                Step::Blocked => {
                    if !was_blocked(body) {
                        changes.push((
                            FactKey::new(body, TRAVEL_BLOCKED),
                            Change::Set(Value::Bool(true)),
                            "way_blocked",
                        ));
                    }
                }
                Step::Arrive => {
                    changes.push((FactKey::new(body, TRAVEL_TO), Change::Tombstone, "arrived"));
                    if segment(view, body).is_some() {
                        for (k, c) in halt(view, body) {
                            changes.push((k, c, "arrived"));
                        }
                    }
                }
                Step::Walk([x, y]) => {
                    let here = local_position(view, body);
                    let frame = container_of(view, body).expect("a traveller is somewhere");
                    // Walk on whatever is underfoot at the waypoint — unless that is more than a
                    // step below: then the walker goes on level, off the edge, and gravity takes
                    // it when the leg ends (a leg is walked through; gravity acts between legs).
                    let below = support_at(
                        view,
                        frame,
                        [x, y, here[2]],
                        self.rules.step_height_cm,
                        &[body],
                    );
                    let z = if here[2] - below > self.rules.step_height_cm {
                        here[2]
                    } else {
                        below
                    };
                    let speed = view
                        .read(FactKey::new(body, TRAVEL_SPEED))
                        .and_then(|f| f.value.as_int())
                        .unwrap_or(0);
                    for (k, c) in depart(view, body, [x, y, z], speed, &self.rules.clock) {
                        changes.push((k, c, "walk"));
                    }
                    if let Some(facing) = compass(x - here[0], y - here[1]) {
                        changes.push((
                            FactKey::new(body, HEADING),
                            Change::Set(Value::Int(facing)),
                            "walk",
                        ));
                    }
                }
                Step::Pass(portal) => {
                    let dest =
                        portal_destination(view, portal).expect("a routed portal leads somewhere");
                    let face = far_side(view, portal, dest);
                    taken.insert(portal);
                    if let Some(q) = face {
                        taken.insert(q);
                    }
                    // Emerge at the far face: its spot in the destination — a window's sill, a
                    // doorway's threshold. Gravity takes it from there if that is off the floor.
                    let emerge = face.map_or([0; 3], |q| local_position(view, q));
                    changes.push((
                        FactKey::new(body, CONTAINED_IN),
                        Change::Set(Value::Entity(dest)),
                        "pass_through",
                    ));
                    changes.push((
                        FactKey::new(body, POSITION),
                        Change::Set(Value::Vec3(emerge)),
                        "pass_through",
                    ));
                    if segment(view, body).is_some() {
                        for fact in [MOTION_TARGET, MOTION_START, MOTION_END] {
                            changes.push((
                                FactKey::new(body, fact),
                                Change::Tombstone,
                                "pass_through",
                            ));
                        }
                    }
                }
            }
            for (key, change, cause) in changes {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    change,
                    Cause::new(cause),
                ));
            }
        }
        out
    }
}
