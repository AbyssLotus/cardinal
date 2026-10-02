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
//!   through an opening to its **far side** — the linked face, or else the nearest face back. If
//!   no such way exists it reports the travel **blocked** ([`TRAVEL_BLOCKED`]), and keeps the
//!   intent: open the door and it goes on. An intent for a body that is not mobile, or with no
//!   speed, is reported blocked too — it can never be carried out.
//! - Within a region, it walks straight when nothing **solid** and taller than a step stands in
//!   the way and the ground is not too steep; otherwise it plans around obstacles on a grid
//!   (A*, eight-connected, never cutting a corner) and walks to the farthest point of that path
//!   it can reach in a straight line. Waypoints sit on the ground (or on a low solid top), and
//!   on terrain no leg is longer than half a sample spacing, so a walker stays on the ground.
//!   Planning state — which entities are places, each region's obstacles, its grid — is built
//!   once per tick and shared by every traveller.
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
use crate::space::{far_side, local_position, portal_destination, rotate, route_where};
use crate::terrain::{ground, is_true, slope_percent, support, support_at};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::isqrt;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::SimClock;
use kernel::value::Value;
use std::collections::{BTreeMap, BTreeSet};

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
    /// How far beyond its own body a body can reach to operate something, in centimetres.
    pub reach_cm: i64,
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

/// A planning grid over one region: which cells a walker of a given radius may stand in. Cell
/// `(i, j)` is centred at `lo + (i, j) · cell + cell/2` in the region's frame.
struct Grid {
    lo: [i64; 2],
    cell: i64,
    nx: i64,
    ny: i64,
    blocked: Vec<bool>,
}

impl Grid {
    fn centre(&self, i: i64, j: i64, z: i64) -> [i64; 3] {
        [
            self.lo[0] + i * self.cell + self.cell / 2,
            self.lo[1] + j * self.cell + self.cell / 2,
            z,
        ]
    }

    fn cell_of(&self, p: [i64; 3]) -> (i64, i64) {
        (
            ((p[0] - self.lo[0]) / self.cell).clamp(0, self.nx - 1),
            ((p[1] - self.lo[1]) / self.cell).clamp(0, self.ny - 1),
        )
    }

    fn is_blocked(&self, i: i64, j: i64) -> bool {
        self.blocked[(j * self.nx + i) as usize]
    }
}

/// Planning state built during one evaluation of [`Travel`] and shared by every traveller in it:
/// which entities are places, each region's tall solids, its terrain cells' slopes, and its
/// planning grids. Nothing here outlives the evaluation — systems hold no state between ticks
/// (Vol. II Ch. 3) — but within a tick a crowd crossing one room plans against one grid instead
/// of each rebuilding it (sweep D8). Every cache is a `BTreeMap`, so nothing depends on hash
/// order.
struct Planner<'v> {
    view: &'v dyn CommittedView,
    rules: MoveRules,
    places: BTreeSet<EntityId>,
    walls: BTreeMap<EntityId, Vec<(EntityId, BodyBox)>>,
    slopes: BTreeMap<(EntityId, i64, i64), Option<i64>>,
    grids: BTreeMap<(EntityId, i64), Grid>,
}

impl<'v> Planner<'v> {
    fn new(view: &'v dyn CommittedView, rules: MoveRules) -> Self {
        // A place is anything one can be *in*: a region with openings, or one an opening leads
        // to. Found once per tick rather than by scanning every portal for every traveller.
        let mut places: BTreeSet<EntityId> = view.entities_with(HAS_PORTAL).into_iter().collect();
        for portal in view.entities_with(LEADS_TO) {
            if let Some(dest) = portal_destination(view, portal) {
                places.insert(dest);
            }
        }
        Self {
            view,
            rules,
            places,
            walls: BTreeMap::new(),
            slopes: BTreeMap::new(),
            grids: BTreeMap::new(),
        }
    }

    /// The solid bodies in `frame` too tall to step onto from the ground beneath them — what a
    /// walker goes around — computed once per frame per tick.
    fn walls_in(&mut self, frame: EntityId) -> Vec<(EntityId, BodyBox)> {
        let view = self.view;
        let step = self.rules.step_height_cm;
        self.walls
            .entry(frame)
            .or_insert_with(|| {
                crate::nearby::contents(view, frame)
                    .into_iter()
                    .filter(|e| is_true(view, *e, SOLID))
                    .filter_map(|e| Some((e, body_box(view, e)?)))
                    .filter(|(_, b)| b.top() > ground(view, frame, b.base[0], b.base[1]) + step)
                    .collect()
            })
            .clone()
    }

    /// The slope of the terrain cell under `(x, y)` in `frame`, cached by terrain cell — the
    /// grade is one value per cell, so every planning cell inside it shares the answer.
    fn slope(&mut self, frame: EntityId, x: i64, y: i64) -> Option<i64> {
        let sp = crate::terrain::spacing(self.view, frame)?;
        let key = (frame, x.div_euclid(sp), y.div_euclid(sp));
        let view = self.view;
        *self
            .slopes
            .entry(key)
            .or_insert_with(|| slope_percent(view, frame, x, y))
    }

    /// Whether a walker of horizontal radius `r` can go straight from `a` to `b` in `frame`: no
    /// obstacle's footprint (grown by `r`) crosses the track, and no ground along it is steeper
    /// than the rules allow.
    fn clear(
        &mut self,
        frame: EntityId,
        walls: &[BodyBox],
        r: i64,
        a: [i64; 3],
        b: [i64; 3],
    ) -> bool {
        if walls.iter().any(|w| w.track_hits(a, b, r)) {
            return false;
        }
        if crate::terrain::spacing(self.view, frame).is_none() {
            return true;
        }
        let run = flat_distance(a, b);
        let samples = (run / self.rules.nav_cell_cm.max(1)).clamp(1, 4096);
        let max = self.rules.max_slope_percent;
        (0..=samples).all(|k| {
            let x = a[0] + (b[0] - a[0]) * k / samples;
            let y = a[1] + (b[1] - a[1]) * k / samples;
            self.slope(frame, x, y).map_or(true, |s| s <= max)
        })
    }

    /// A grid over `lo..hi` of `frame`, with every cell blocked that lies inside one of `walls`'
    /// footprints (grown by `r`) or on ground steeper than the rules allow. Obstacles are
    /// stamped onto only the cells under their own extent, so building costs the obstacles'
    /// area, not cells × obstacles.
    fn build_grid(
        &mut self,
        frame: EntityId,
        lo: [i64; 2],
        hi: [i64; 2],
        walls: &[BodyBox],
        r: i64,
    ) -> Grid {
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1);
        let cell = self
            .rules
            .nav_cell_cm
            .max(1)
            .max((span + MAX_GRID - 1) / MAX_GRID);
        let (nx, ny) = ((hi[0] - lo[0]) / cell + 1, (hi[1] - lo[1]) / cell + 1);
        let mut grid = Grid {
            lo,
            cell,
            nx,
            ny,
            blocked: vec![false; (nx * ny) as usize],
        };
        for w in walls {
            // The turned footprint's enclosing box, grown by r, in grid cells.
            let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
            for (cx, cy) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
                let c = rotate(w.heading, [cx * (w.size[0] + r), cy * (w.size[1] + r), 0]);
                x0 = x0.min(w.base[0] + c[0]);
                x1 = x1.max(w.base[0] + c[0]);
                y0 = y0.min(w.base[1] + c[1]);
                y1 = y1.max(w.base[1] + c[1]);
            }
            let (i0, j0) = grid.cell_of([x0, y0, 0]);
            let (i1, j1) = grid.cell_of([x1, y1, 0]);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    if w.footprint_contains(grid.centre(i, j, w.base[2]), r) {
                        grid.blocked[(j * nx + i) as usize] = true;
                    }
                }
            }
        }
        if crate::terrain::spacing(self.view, frame).is_some() {
            let max = self.rules.max_slope_percent;
            for j in 0..ny {
                for i in 0..nx {
                    let c = grid.centre(i, j, 0);
                    if self.slope(frame, c[0], c[1]).is_some_and(|s| s > max) {
                        grid.blocked[(j * nx + i) as usize] = true;
                    }
                }
            }
        }
        grid
    }

    /// The next point a walker standing at `start` in `frame` should walk straight to on its way
    /// to `goal`, stopping `stop` short of it: the goal itself if the way is clear, else the
    /// farthest clear point along a planned path around the obstacles. `None` if no path exists.
    /// `target` is the thing walked up to, if any — not an obstacle to its own approach.
    fn next_waypoint(
        &mut self,
        frame: EntityId,
        walker: EntityId,
        start: [i64; 3],
        goal: [i64; 3],
        stop: i64,
        target: Option<EntityId>,
    ) -> Option<[i64; 2]> {
        let r = radius(self.view, walker);
        let step = self.rules.step_height_cm;
        // Obstacles for this walker: the region's tall solids, less the walker itself, what it is
        // walking up to, and whatever it is standing on (someone on a shed's roof walks across
        // it, and off its edge).
        let all = self.walls_in(frame);
        let walls: Vec<BodyBox> = all
            .iter()
            .filter(|(e, b)| {
                *e != walker
                    && Some(*e) != target
                    && !(b.footprint_contains(start, 0) && b.top() <= start[2] + step)
            })
            .map(|(_, b)| *b)
            .collect();
        let personal = walls.len() != all.len();

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
        if self.clear(frame, &walls, r, start, end3) {
            return Some(end);
        }

        // Plan on a grid over the region — its size, if it has one; else the ground its terrain
        // covers; else the area around the walker, the goal, and the obstacles. A region with
        // fixed bounds and no walker-specific exclusions shares one grid per walker size.
        let fixed = match (size_of(self.view, frame), terrain_extent(self.view, frame)) {
            (Some(s), _) => Some(([-s[0], -s[1]], [s[0], s[1]])),
            (None, Some(far)) => Some(([0, 0], far)),
            (None, None) => None,
        };
        let shared_key = (frame, r);
        let grid = match fixed {
            Some(_) if !personal && self.grids.contains_key(&shared_key) => {
                self.grids.remove(&shared_key).expect("checked present")
            }
            Some((lo, hi)) if !personal => {
                // Shared: the region's own bounds only, never stretched to fit one walker (a
                // point outside them plans from the nearest edge cell).
                self.build_grid(frame, lo, hi, &walls, r)
            }
            Some((lo, hi)) => {
                let lo = [
                    lo[0].min(start[0]).min(goal[0]),
                    lo[1].min(start[1]).min(goal[1]),
                ];
                let hi = [
                    hi[0].max(start[0]).max(goal[0]),
                    hi[1].max(start[1]).max(goal[1]),
                ];
                self.build_grid(frame, lo, hi, &walls, r)
            }
            None => {
                let mut lo = [start[0].min(goal[0]), start[1].min(goal[1])];
                let mut hi = [start[0].max(goal[0]), start[1].max(goal[1])];
                for w in &walls {
                    let far = w.size[0].max(w.size[1]) + r;
                    lo = [lo[0].min(w.base[0] - far), lo[1].min(w.base[1] - far)];
                    hi = [hi[0].max(w.base[0] + far), hi[1].max(w.base[1] + far)];
                }
                let pad = 4 * self.rules.nav_cell_cm.max(1);
                self.build_grid(
                    frame,
                    [lo[0] - pad, lo[1] - pad],
                    [hi[0] + pad, hi[1] + pad],
                    &walls,
                    r,
                )
            }
        };
        let (si, sj) = grid.cell_of(start);
        let (gi, gj) = grid.cell_of(goal);
        let blocked =
            |i: i64, j: i64| (i, j) != (si, sj) && (i, j) != (gi, gj) && grid.is_blocked(i, j);
        let path = astar(grid.nx, grid.ny, (si, sj), (gi, gj), blocked);
        let mut answer = None;
        if let Some(path) = &path {
            // Walk to the farthest point of the path reachable in a straight line.
            for &(i, j) in path.iter().rev() {
                let target_pt = if (i, j) == (gi, gj) {
                    end3
                } else {
                    grid.centre(i, j, start[2])
                };
                if self.clear(frame, &walls, r, start, target_pt) {
                    answer = Some([target_pt[0], target_pt[1]]);
                    break;
                }
            }
            // Failing that, the first step of the path is adjacent and clear of corners.
            if answer.is_none() {
                answer = path.get(1).map(|&(i, j)| {
                    let c = grid.centre(i, j, start[2]);
                    [c[0], c[1]]
                });
            }
        }
        if fixed.is_some() && !personal {
            self.grids.insert(shared_key, grid);
        }
        if path.is_some() {
            answer
        } else {
            None
        }
    }
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
    fn step_for(&self, plan: &mut Planner, body: EntityId, taken: &BTreeSet<EntityId>) -> Step {
        let view = plan.view;
        let Some(Value::Entity(target)) = view.read(FactKey::new(body, TRAVEL_TO)).map(|f| f.value)
        else {
            return Step::Wait;
        };
        let speed = view
            .read(FactKey::new(body, TRAVEL_SPEED))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0);
        // Only a free body can travel, and only at some speed: an intent without either can
        // never be carried out, and the decider is told so rather than left waiting (sweep D5).
        if speed <= 0 || !is_true(view, body, MOBILE) {
            return Step::Blocked;
        }
        let step = self.rules.step_height_cm;
        if is_moving(view, body) || !supported(view, body, step) {
            return Step::Wait;
        }
        let Some(here) = container_of(view, body) else {
            return Step::Blocked;
        };
        let at = local_position(view, body);

        // Where must it be? Inside a place, or beside a thing.
        let (goal_region, approach) = if plan.places.contains(&target) {
            (target, None)
        } else {
            let Some(region) = container_of(view, target) else {
                return Step::Blocked;
            };
            (region, Some(target))
        };
        let walk =
            |plan: &mut Planner, goal: [i64; 3], stop: i64, thing: Option<EntityId>| match plan
                .next_waypoint(here, body, at, goal, stop, thing)
            {
                Some(w) => Step::Walk(cap_on_terrain(view, here, at, w)),
                None => Step::Blocked,
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
            return walk(plan, goal, reach - ARRIVAL_TOLERANCE, Some(thing));
        }

        // Another region: the way out, through openings it fits that are open and in reach.
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
        walk(plan, door, 0, None)
    }
}

/// On terrain, a leg is at most half a sample spacing long. A leg is a straight line and ground
/// is curved, so a long leg over a hill would cut through it or float above a hollow; short legs
/// keep a walker on the ground (sweep D10) at the cost of a few more writes per journey.
fn cap_on_terrain(
    view: &dyn CommittedView,
    frame: EntityId,
    at: [i64; 3],
    w: [i64; 2],
) -> [i64; 2] {
    let Some(sp) = crate::terrain::spacing(view, frame) else {
        return w;
    };
    let limit = (sp / 2).max(1);
    let len = flat_distance(at, [w[0], w[1], at[2]]);
    if len <= limit {
        return w;
    }
    [
        at[0] + ((w[0] - at[0]) as i128 * limit as i128 / len as i128) as i64,
        at[1] + ((w[1] - at[1]) as i128 * limit as i128 / len as i128) as i64,
    ]
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
        let mut plan = Planner::new(view, self.rules);
        let was_blocked = |b: EntityId| is_true(view, b, TRAVEL_BLOCKED);
        // Ascending id: when two bodies reach one opening on one tick, the lower id goes first.
        for body in view.entities_with(TRAVEL_TO) {
            let mut changes: Vec<(FactKey, Change, &'static str)> = Vec::new();
            let step = self.step_for(&mut plan, body, &taken);
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
                    // Walk on whatever is underfoot at the waypoint. The one exception is a
                    // walker standing on top of something solid — a roof, a boulder — whose
                    // waypoint lies more than a step below: it goes on level, off the edge, and
                    // gravity takes it when the leg ends (a leg is walked through; gravity acts
                    // between legs). Walking down a slope is not a ledge: the planner has already
                    // judged the ground walkable, so the walker follows it down.
                    let step = self.rules.step_height_cm;
                    let below = support_at(view, frame, [x, y, here[2]], step, &[body]);
                    let on_top_of_something =
                        support(view, body, step) > ground(view, frame, here[0], here[1]) + 1;
                    let z = if on_top_of_something && here[2] - below > step {
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
