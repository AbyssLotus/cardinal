//! The validation stack, for the world-file format (Vol. IV Ch. 7 §7.1).
//!
//! Validation runs before anything is seeded, reads only the package, and fixes nothing — a
//! validator that repaired what it found would be an unchronicled writer (§7.1, the one rule
//! spanning all layers). It reports *every* problem it finds, each naming its layer, its subject,
//! and its rule, so an author fixes a file in one pass rather than one error at a time.
//!
//! - **Layer 2 — Reference.** Every entity referenced resolves to one the package declares
//!   (appears as the subject of some declaration), and no fact is declared twice for one entity —
//!   within a section, or across sections that seed the same fact (an entity's container comes
//!   from `[containment]`, `[organisms]`, and `[portals]`; its position from `[positions]` and
//!   `[portals]`, `[places]`). Seeding a single-valued fact twice would give the entity two values and every
//!   later query an arbitrary one (sweep D1). Overlapping regions named in `[in_region]` are the
//!   one exception to "declared before referenced": a classification region is *made* by being
//!   named (Vol. III Ch. 1 §1.7).
//! - **Layer 3 — Coherence.** Declarations make joint sense: a `closed` flag belongs on a portal;
//!   a portal is paired with one other portal, not with itself.
//! - **Layer 4 — World.** The generated reality is well-formed: containment is a hierarchy, with
//!   no entity inside itself by any chain.

use crate::model::{Flag, WorldPackage};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Which validation layer a problem belongs to (Vol. IV Ch. 7 §7.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Layer {
    /// Layer 2: references resolve; nothing is declared twice.
    Reference,
    /// Layer 3: declarations make joint sense.
    Coherence,
    /// Layer 4: the world the package generates is well-formed.
    World,
}

/// One problem found in a package: its layer, the entity it concerns, and the rule it breaks.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Problem {
    /// The validation layer.
    pub layer: Layer,
    /// The entity concerned (raw id).
    pub subject: u64,
    /// The rule broken, in words.
    pub rule: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let layer = match self.layer {
            Layer::Reference => "layer 2 (reference)",
            Layer::Coherence => "layer 3 (coherence)",
            Layer::World => "layer 4 (world)",
        };
        write!(f, "{layer}: entity {}: {}", self.subject, self.rule)
    }
}

/// Validate `package`, returning every problem found (empty if it is sound), sorted by layer and
/// subject so the report reads in a stable order.
pub fn validate(package: &WorldPackage) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut problem = |layer, subject, rule: String| {
        problems.push(Problem {
            layer,
            subject,
            rule,
        })
    };

    // Everything the package declares: the subject of any declaration.
    let mut declared: BTreeSet<u64> = BTreeSet::new();
    declared.extend(package.regions.iter().map(|r| r.id));
    declared.extend(package.organisms.iter().map(|o| o.id));
    declared.extend(package.containment.iter().map(|c| c.child_id));
    declared.extend(package.positions.iter().map(|p| p.entity_id));
    declared.extend(package.portals.iter().map(|p| p.portal_id));
    declared.extend(package.materials.iter().map(|m| m.id));
    declared.extend(package.made_of.iter().map(|m| m.object_id));
    declared.extend(package.bodies.iter().map(|b| b.entity_id));
    declared.extend(package.facing.iter().map(|f| f.entity_id));
    declared.extend(package.motion.iter().map(|m| m.entity_id));
    declared.extend(package.flags.iter().map(|f| f.entity_id));
    declared.extend(package.terrain.iter().map(|t| t.region_id));
    declared.extend(package.travel.iter().map(|t| t.entity_id));
    declared.extend(package.exposure.iter().map(|x| x.region_id));
    declared.extend(package.in_region.iter().map(|m| m.location_id));
    declared.extend(package.places.iter().map(|(p, _)| *p));
    // Recipes and jobs are entities too, which a mind can be told of (Amendments A-17, A-18).
    declared.extend(package.recipes.iter().map(|r| r.id));
    declared.extend(package.jobs.iter().map(|j| j.id));
    // Classification regions are made by being named (§1.7).
    declared.extend(package.in_region.iter().map(|m| m.region_id));
    let portals: BTreeSet<u64> = package.portals.iter().map(|p| p.portal_id).collect();
    let materials: BTreeSet<u64> = package.materials.iter().map(|m| m.id).collect();

    // ---- Layer 2: references resolve.
    let mut must_exist = |subject: u64, target: u64, what: &str| {
        if !declared.contains(&target) {
            problem(
                Layer::Reference,
                subject,
                format!("{what} {target} is never declared"),
            );
        }
    };
    for c in &package.containment {
        must_exist(c.child_id, c.parent_id, "its container");
    }
    for o in &package.organisms {
        must_exist(o.id, o.region_id, "its region");
    }
    for &(place, within) in &package.places {
        if let Some(parent) = within {
            must_exist(place, parent, "its container");
        }
    }
    for p in &package.portals {
        must_exist(p.portal_id, p.host_region, "its host region");
        must_exist(p.portal_id, p.dest_region, "its destination");
    }
    for a in &package.adjacency {
        must_exist(a.a, a.b, "its neighbour");
        must_exist(a.b, a.a, "its neighbour");
    }
    for t in &package.travel {
        must_exist(t.entity_id, t.target, "its travel destination");
    }
    for &(organism, _) in &package.senses {
        must_exist(organism, organism, "the organism given sight");
    }
    for &(organism, _) in &package.fatigue {
        must_exist(organism, organism, "the organism given fatigue");
    }
    for &(who, _, _) in &package.dependence {
        must_exist(who, who, "the organism given a dependence");
    }
    for &(who, _) in &package.temperament {
        must_exist(who, who, "the mind given a temperament");
    }
    for &(who, _) in &package.minds {
        must_exist(who, who, "the entity given a mind");
    }
    for &(who, _, _, target) in &package.routines {
        must_exist(who, who, "the mind given a routine");
        must_exist(who, target, "the place its routine takes it to");
    }
    for r in &package.recipes {
        must_exist(r.id, r.at, "the workplace it is made at");
    }
    for j in &package.jobs {
        must_exist(j.id, j.to, "the store it keeps");
        if let crate::model::JobWork::Carry { from, .. } = j.work {
            must_exist(j.id, from, "where it gets its goods");
        }
    }
    for &(who, _) in &package.roles {
        must_exist(who, who, "the person given a role");
    }
    for &(thing, owner) in &package.owners {
        must_exist(thing, thing, "the thing given an owner");
        must_exist(thing, owner, "its owner");
    }
    for &(who, _) in &package.curiosity {
        must_exist(who, who, "the mind given curiosity");
    }
    for &(who, _, _) in &package.likes {
        must_exist(who, who, "the mind given likes");
    }
    for &(who, place) in &package.home {
        must_exist(who, who, "the mind given a home");
        must_exist(who, place, "its home");
    }
    for (mind, things) in &package.knows {
        must_exist(*mind, *mind, "the mind given knowledge");
        for &thing in things {
            must_exist(*mind, thing, "the thing it knows of");
        }
    }
    for m in &package.made_of {
        if !materials.contains(&m.material_id) {
            problem(
                Layer::Reference,
                m.object_id,
                format!("material {} is not declared in [materials]", m.material_id),
            );
        }
    }
    for &(who, material, _) in &package.dependence {
        if !materials.contains(&material) {
            problem(
                Layer::Reference,
                who,
                format!("material {material} is not declared in [materials]"),
            );
        }
    }
    for j in &package.jobs {
        match j.work {
            crate::model::JobWork::Carry { material, .. } if !materials.contains(&material) => {
                problem(
                    Layer::Reference,
                    j.id,
                    format!("material {material} is not declared in [materials]"),
                );
            }
            crate::model::JobWork::Make { recipe }
                if !package.recipes.iter().any(|r| r.id == recipe) =>
            {
                problem(
                    Layer::Reference,
                    j.id,
                    format!("recipe {recipe} is not declared in [recipes]"),
                );
            }
            _ => {}
        }
    }
    for &(who, material, _) in &package.likes {
        if !materials.contains(&material) {
            problem(
                Layer::Reference,
                who,
                format!("material {material} is not declared in [materials]"),
            );
        }
    }
    for &(who, job) in &package.roles {
        if !package.jobs.iter().any(|j| j.id == job) {
            problem(
                Layer::Reference,
                who,
                format!("job {job} is not declared in [jobs]"),
            );
        }
    }
    for r in &package.recipes {
        for material in r.needs.iter().map(|(m, _)| *m).chain([r.makes]) {
            if !materials.contains(&material) {
                problem(
                    Layer::Reference,
                    r.id,
                    format!("material {material} is not declared in [materials]"),
                );
            }
        }
    }
    for d in &package.deposits {
        if !materials.contains(&d.made_of) {
            problem(
                Layer::Reference,
                d.id,
                format!("material {} is not declared in [materials]", d.made_of),
            );
        }
    }
    for d in &package.portal_danger {
        if !portals.contains(&d.portal_id) {
            problem(
                Layer::Reference,
                d.portal_id,
                "[portal_danger] names something that is not a portal".into(),
            );
        }
    }

    // ---- Layer 2: nothing declared twice. Each single-valued fact, with every section that
    // seeds it.
    let mut once = |fact: &str, sources: Vec<(&str, u64)>| {
        let mut seen: BTreeMap<u64, &str> = BTreeMap::new();
        for (section, id) in sources {
            if let Some(first) = seen.insert(id, section) {
                problem(
                    Layer::Reference,
                    id,
                    format!("its {fact} is declared twice (in {first} and in {section})"),
                );
            }
        }
    };
    once(
        "container",
        package
            .containment
            .iter()
            .map(|c| ("[containment]", c.child_id))
            .chain(package.organisms.iter().map(|o| ("[organisms]", o.id)))
            .chain(package.portals.iter().map(|p| ("[portals]", p.portal_id)))
            .chain(package.places.iter().map(|(p, _)| ("[places]", *p)))
            .collect(),
    );
    once(
        "position",
        package
            .positions
            .iter()
            .map(|p| ("[positions]", p.entity_id))
            .chain(package.portals.iter().map(|p| ("[portals]", p.portal_id)))
            .collect(),
    );
    let single = |section: &'static str, ids: Vec<u64>| -> Vec<(&'static str, u64)> {
        ids.into_iter().map(|id| (section, id)).collect()
    };
    once(
        "climate",
        single("[regions]", package.regions.iter().map(|r| r.id).collect()),
    );
    once(
        "size",
        single(
            "[bodies]",
            package.bodies.iter().map(|b| b.entity_id).collect(),
        ),
    );
    once(
        "facing",
        single(
            "[facing]",
            package.facing.iter().map(|f| f.entity_id).collect(),
        ),
    );
    once(
        "motion",
        single(
            "[motion]",
            package.motion.iter().map(|m| m.entity_id).collect(),
        ),
    );
    once(
        "terrain",
        single(
            "[terrain]",
            package.terrain.iter().map(|t| t.region_id).collect(),
        ),
    );
    once(
        "travel intent",
        single(
            "[travel]",
            package.travel.iter().map(|t| t.entity_id).collect(),
        ),
    );
    once(
        "exposure",
        single(
            "[exposure]",
            package.exposure.iter().map(|x| x.region_id).collect(),
        ),
    );
    once(
        "pinned danger",
        single(
            "[portal_danger]",
            package.portal_danger.iter().map(|d| d.portal_id).collect(),
        ),
    );
    once(
        "starting fatigue",
        single("[fatigue]", package.fatigue.iter().map(|f| f.0).collect()),
    );
    once(
        "temperament",
        single(
            "[temperament]",
            package.temperament.iter().map(|t| t.0).collect(),
        ),
    );
    once(
        "need kind",
        single(
            "[need_kinds]",
            package.need_kinds.iter().map(|k| k.id).collect(),
        ),
    );
    once(
        "mind",
        single("[minds]", package.minds.iter().map(|m| m.0).collect()),
    );
    once(
        "sight range",
        single("[senses]", package.senses.iter().map(|s| s.0).collect()),
    );
    once(
        "material properties",
        single(
            "[materials]",
            package.materials.iter().map(|m| m.id).collect(),
        ),
    );

    // ---- Layer 3: coherence.
    for spec in &package.flags {
        if spec.flags.contains(&Flag::Closed) && !portals.contains(&spec.entity_id) {
            problem(
                Layer::Coherence,
                spec.entity_id,
                "only a portal can be closed".into(),
            );
        }
    }
    let mut partner: BTreeMap<u64, u64> = BTreeMap::new();
    for &(a, b) in &package.portal_pairs {
        for (x, y) in [(a, b), (b, a)] {
            if !portals.contains(&x) {
                problem(
                    Layer::Reference,
                    x,
                    "[portal_pairs] names something that is not a portal".into(),
                );
                continue;
            }
            if x == y {
                problem(
                    Layer::Coherence,
                    x,
                    "a portal cannot be its own far side".into(),
                );
                continue;
            }
            if let Some(other) = partner.insert(x, y) {
                if other != y {
                    problem(
                        Layer::Coherence,
                        x,
                        format!("paired with both {other} and {y}; an opening has two faces"),
                    );
                }
            }
        }
    }

    // ---- Layer 4: containment is a hierarchy. Walk up from every entity; meeting the start
    // again is a cycle (reported once, at its lowest id).
    let mut parent: BTreeMap<u64, u64> = BTreeMap::new();
    for c in &package.containment {
        parent.insert(c.child_id, c.parent_id);
    }
    for o in &package.organisms {
        parent.insert(o.id, o.region_id);
    }
    for p in &package.portals {
        parent.insert(p.portal_id, p.host_region);
    }
    for &(place, within) in &package.places {
        if let Some(up) = within {
            parent.insert(place, up);
        }
    }
    let mut reported: BTreeSet<u64> = BTreeSet::new();
    for &start in parent.keys() {
        let mut here = start;
        let mut path = vec![start];
        while let Some(&up) = parent.get(&here) {
            if up == start {
                let lowest = *path.iter().min().expect("path holds the start");
                if reported.insert(lowest) {
                    let mut ring = path.clone();
                    ring.sort_unstable();
                    problem(
                        Layer::World,
                        lowest,
                        format!("containment loops back on itself through {ring:?}"),
                    );
                }
                break;
            }
            if path.contains(&up) || path.len() > parent.len() {
                break; // a loop that does not pass through `start`; found from its own members
            }
            path.push(up);
            here = up;
        }
    }

    problems.sort_by(|a, b| (a.layer, a.subject, &a.rule).cmp(&(b.layer, b.subject, &b.rule)));
    problems
}
