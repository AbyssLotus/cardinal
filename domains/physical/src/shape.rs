//! Exact geometric tests against bodies (Vol. III Ch. 1 §1.11; Amendment A-4): is a point
//! inside a body's footprint, does a line pass through a body.
//!
//! A body is a box: `half_width` to either side of its base along its own x axis, `half_depth`
//! fore and aft along its own y, `height` upward — turned by its heading ([`crate::schema`]
//! `BODY_SIZE`, `HEADING`). The spatial index files bodies by the axis-aligned box that encloses
//! the turned one (good for finding candidates); the tests here are exact, made in the body's
//! own frame: the point or line is shifted to the body's base and turned back by its heading,
//! then compared with the upright box.
//!
//! The line test is the classic slab method in exact rational arithmetic: each axis narrows the
//! interval of the segment's parameter `t ∈ [0, 1]` that lies within the box's slab, as
//! fractions compared by cross-multiplication in `i128`, so no rounding can admit a miss or
//! reject a hit (Vol. V Ch. 4 §4.1).

use crate::index::size_of;
use crate::space::{heading, local_position, unrotate};
use kernel::identity::EntityId;
use kernel::system::CommittedView;

/// A body's box, in the body's own (upright, unturned) frame, measured from its base.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BodyBox {
    /// Where the body's base stands, in its container's frame.
    pub base: [i64; 3],
    /// Its heading in its container's frame.
    pub heading: i64,
    /// `[half_width, half_depth, height]`.
    pub size: [i64; 3],
}

impl BodyBox {
    /// `p` (container frame) in the body's own frame, measured from its base.
    pub fn to_local(&self, p: [i64; 3]) -> [i64; 3] {
        unrotate(
            self.heading,
            [
                p[0] - self.base[0],
                p[1] - self.base[1],
                p[2] - self.base[2],
            ],
        )
    }

    /// Whether `p` (container frame) lies over the body's footprint, grown by `margin` on every
    /// side — heights ignored.
    pub fn footprint_contains(&self, p: [i64; 3], margin: i64) -> bool {
        let l = self.to_local(p);
        l[0].abs() <= self.size[0] + margin && l[1].abs() <= self.size[1] + margin
    }

    /// The height of the body's top, in its container's frame.
    pub fn top(&self) -> i64 {
        self.base[2].saturating_add(self.size[2])
    }

    /// Whether the straight segment from `a` to `b` (container frame) passes through the box,
    /// grown by `margin` across its footprint (not in height). Touching counts.
    pub fn segment_hits(&self, a: [i64; 3], b: [i64; 3], margin: i64) -> bool {
        let la = self.to_local(a);
        let lb = self.to_local(b);
        let lo = [-(self.size[0] + margin), -(self.size[1] + margin), 0];
        let hi = [self.size[0] + margin, self.size[1] + margin, self.size[2]];
        segment_hits_box(la, lb, lo, hi, 3)
    }

    /// As [`BodyBox::segment_hits`], ignoring heights entirely: does the segment's track across
    /// the ground cross the footprint?
    pub fn track_hits(&self, a: [i64; 3], b: [i64; 3], margin: i64) -> bool {
        let la = self.to_local(a);
        let lb = self.to_local(b);
        let lo = [-(self.size[0] + margin), -(self.size[1] + margin), 0];
        let hi = [self.size[0] + margin, self.size[1] + margin, 0];
        segment_hits_box(la, lb, lo, hi, 2)
    }
}

/// The box of `entity` at the view's tick, if it has a size.
pub fn body_box(view: &dyn CommittedView, entity: EntityId) -> Option<BodyBox> {
    Some(BodyBox {
        base: local_position(view, entity),
        heading: heading(view, entity),
        size: size_of(view, entity)?,
    })
}

/// A fraction `num / den` with `den > 0`.
#[derive(Clone, Copy)]
struct Frac {
    num: i128,
    den: i128,
}

impl Frac {
    fn le(self, other: Frac) -> bool {
        self.num * other.den <= other.num * self.den
    }
}

/// Whether the segment `a → b` meets the axis-aligned box `[lo, hi]` in the first `axes`
/// dimensions (2: a footprint, 3: a volume). Bounds inclusive.
fn segment_hits_box(a: [i64; 3], b: [i64; 3], lo: [i64; 3], hi: [i64; 3], axes: usize) -> bool {
    let mut t_min = Frac { num: 0, den: 1 };
    let mut t_max = Frac { num: 1, den: 1 };
    for i in 0..axes {
        let (p, d) = (a[i] as i128, b[i] as i128 - a[i] as i128);
        let (lo, hi) = (lo[i] as i128, hi[i] as i128);
        if d == 0 {
            if p < lo || p > hi {
                return false;
            }
            continue;
        }
        // Entering and leaving this slab, as fractions with positive denominators.
        let (near, far) = if d > 0 {
            (
                Frac {
                    num: lo - p,
                    den: d,
                },
                Frac {
                    num: hi - p,
                    den: d,
                },
            )
        } else {
            (
                Frac {
                    num: p - hi,
                    den: -d,
                },
                Frac {
                    num: p - lo,
                    den: -d,
                },
            )
        };
        if !near.le(t_min) {
            t_min = near;
        }
        if !t_max.le(far) {
            t_max = far;
        }
        if !t_min.le(t_max) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::BodyBox;

    fn table() -> BodyBox {
        // A 2 m × 1 m table, 80 cm high, at the origin, unturned.
        BodyBox {
            base: [0, 0, 0],
            heading: 0,
            size: [100, 50, 80],
        }
    }

    #[test]
    fn footprints_turn_with_their_bodies() {
        let mut t = table();
        assert!(t.footprint_contains([90, 0, 0], 0));
        assert!(!t.footprint_contains([0, 90, 0], 0));
        t.heading = 9_000; // now its long side runs north–south
        assert!(!t.footprint_contains([90, 0, 0], 0));
        assert!(t.footprint_contains([0, 90, 0], 0));
        assert!(t.footprint_contains([60, 0, 0], 15), "a margin grows it");
    }

    #[test]
    fn segments_hit_or_miss_exactly() {
        let t = table();
        // Straight across the tabletop's height: hit.
        assert!(t.segment_hits([-500, 0, 40], [500, 0, 40], 0));
        // Over it: miss. Grazing its top edge: hit (touching counts).
        assert!(!t.segment_hits([-500, 0, 81], [500, 0, 81], 0));
        assert!(t.segment_hits([-500, 0, 80], [500, 0, 80], 0));
        // Stopping short of it: miss.
        assert!(!t.segment_hits([-500, 0, 40], [-101, 0, 40], 0));
        // A diagonal that clips a corner: hit; one that passes outside it: miss.
        assert!(t.segment_hits([-200, -100, 10], [0, 100, 10], 0));
        assert!(!t.segment_hits([-200, 0, 10], [-100, 100, 10], 0));
        // The track ignores height: passing over the table still crosses its footprint.
        assert!(t.track_hits([-500, 0, 500], [500, 0, 500], 0));
    }
}
