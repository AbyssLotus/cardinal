//! Deterministic fixed-point arithmetic — Vol. V Ch. 4 §4.1 (Level 2: committed state is
//! computed in integers, never floats).
//!
//! Every domain needs the same few operations — a square root, a rounded or unbiased
//! division, and (for space) sine and cosine — and every one of them must give bit-identical
//! answers on every platform. Floating-point `sqrt` is correctly rounded, but `sin`, `cos`, and
//! their friends are not: two libm implementations may disagree in the last bit, and one bit is
//! a divergent replay (Vol. V Ch. 4, Door 3). So the kernel supplies them once, in integer
//! arithmetic, and domains use these rather than reaching for `f64`.
//!
//! This module holds mechanism only. It knows no unit, no scale a world chose, and no
//! quantity's meaning (Vol. IV Ch. 1 §1.5.1).

use crate::rng::Rng;

/// Floor of the square root of `n`, exactly, by Newton's method on integers.
pub fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    // Start above the root (2^(ceil(bits/2))) and descend; Newton's iteration on integers is
    // monotone from above, so the first non-decreasing step is the floor root.
    let bits = 128 - n.leading_zeros();
    let mut x: u128 = 1 << bits.div_ceil(2);
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// `a / b` rounded to the nearest integer, halves away from zero. `b` must be positive.
pub fn div_round(a: i128, b: i128) -> i128 {
    debug_assert!(b > 0, "div_round divisor must be positive");
    if a >= 0 {
        (a + b / 2) / b
    } else {
        -((-a + b / 2) / b)
    }
}

/// `a / b` rounded **without bias**: down or up, with probability equal to the fractional
/// part, decided by the caller's deterministic stream. `b` must be positive and fit in `u64`.
///
/// Why it exists (Vol. II Ch. 2, *Simulated Duration*): at fine tick lengths a per-step change
/// can be smaller than a quantity's smallest unit. Ordinary rounding then turns it into zero
/// every step, and the change never happens — a body that should cool toward the air stops a
/// degree short forever. Rounding up with probability equal to the remainder keeps the expected
/// change exact, so the quantity follows its rule on average at any resolution, and the stream
/// keeps it reproducible.
pub fn div_dither(a: i128, b: i128, rng: &mut Rng) -> i128 {
    debug_assert!(
        b > 0 && b <= u64::MAX as i128,
        "div_dither divisor out of range"
    );
    let q = a.div_euclid(b);
    let r = a.rem_euclid(b); // 0 <= r < b
    if r != 0 && (rng.below(b as u64) as i128) < r {
        q + 1
    } else {
        q
    }
}

/// Fixed-point scale of [`sin_cos`]'s results: 1.0 is `1 << 30`.
pub const TRIG_ONE: i64 = 1 << 30;

/// Angles are measured in hundredths of a degree; a full turn is this many.
pub const FULL_TURN: i64 = 36_000;

/// Sine and cosine of `angle` (hundredths of a degree, any value — it wraps), each scaled by
/// [`TRIG_ONE`]. Exact to within a few units of the last place across the whole circle, and —
/// what matters more — identical on every platform.
///
/// The angle is folded into the first octant (0°–45°) by symmetry, converted to radians in
/// Q62 fixed point, and evaluated with Taylor series that, on that small interval, converge
/// well past 30-bit precision within a handful of terms. Everything is `i128` integer
/// arithmetic; no floating point is involved anywhere.
pub fn sin_cos(angle: i64) -> (i64, i64) {
    let a = angle.rem_euclid(FULL_TURN);
    // Quadrant and the angle within it.
    let quadrant = a / 9_000;
    let within = a % 9_000;
    // Within a quadrant, fold the upper half onto the lower: sin(90° − x) = cos(x).
    let (s, c) = if within <= 4_500 {
        octant(within)
    } else {
        let (s, c) = octant(9_000 - within);
        (c, s)
    };
    match quadrant {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

/// Sine and cosine for an angle in `0..=4500` hundredths of a degree (the first octant), scaled
/// by [`TRIG_ONE`].
fn octant(centideg: i64) -> (i64, i64) {
    // x in radians, Q62: x = centideg * π / 18000. π in Q62 (rounded).
    const PI_Q62: i128 = 14_488_038_916_154_245_685;
    const ONE_Q62: i128 = 1 << 62;
    let x = (centideg as i128 * PI_Q62) / 18_000;
    // x <= π/4 < 1, so x² and every later power stay below ONE_Q62: no overflow in i128.
    let x2 = (x * x) >> 62;
    // sin x = x − x³/3! + x⁵/5! − …, cos x = 1 − x²/2! + x⁴/4! − …, each term built from the
    // previous by one multiply and one small division. Fourteen terms exceed what Q30 needs.
    let mut sin = x;
    let mut term = x;
    for k in 1..=7i128 {
        term = -((term * x2) >> 62) / ((2 * k) * (2 * k + 1));
        sin += term;
    }
    let mut cos = ONE_Q62;
    let mut term = ONE_Q62;
    for k in 1..=7i128 {
        term = -((term * x2) >> 62) / ((2 * k - 1) * (2 * k));
        cos += term;
    }
    // Q62 -> Q30, rounded.
    let to_q30 = |v: i128| ((v + (1 << 31)) >> 32) as i64;
    (to_q30(sin), to_q30(cos))
}

#[cfg(test)]
mod tests {
    use super::{div_dither, div_round, isqrt, sin_cos, TRIG_ONE};
    use crate::rng::{Rng, SubstreamKey};

    #[test]
    fn isqrt_is_the_exact_floor_root() {
        for n in [
            0u128,
            1,
            2,
            3,
            4,
            15,
            16,
            17,
            99,
            100,
            101,
            1 << 64,
            (1 << 100) + 12345,
        ] {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "isqrt({n}) = {r}");
        }
        assert_eq!(isqrt(u128::MAX), (1u128 << 64) - 1);
    }

    #[test]
    fn div_round_rounds_half_away_from_zero() {
        assert_eq!(div_round(5, 2), 3);
        assert_eq!(div_round(-5, 2), -3);
        assert_eq!(div_round(4, 3), 1);
        assert_eq!(div_round(-4, 3), -1);
    }

    #[test]
    fn dithered_division_is_unbiased_and_deterministic() {
        // 1/10 rounds to zero every time with ordinary rounding; dithered, ten thousand of them
        // sum to about a thousand.
        let mut rng = Rng::for_substream(7, SubstreamKey::new(1, 1, 1));
        let total: i128 = (0..10_000).map(|_| div_dither(1, 10, &mut rng)).sum();
        assert!((900..=1100).contains(&total), "sum {total}");
        // Negative values dither between floor and ceil the same way.
        let total: i128 = (0..10_000).map(|_| div_dither(-1, 10, &mut rng)).sum();
        assert!((-1100..=-900).contains(&total), "sum {total}");
        // Exact divisions never dither.
        assert_eq!(div_dither(30, 10, &mut rng), 3);
        // Same stream, same answers.
        let mut a = Rng::for_substream(9, SubstreamKey::new(2, 3, 4));
        let mut b = Rng::for_substream(9, SubstreamKey::new(2, 3, 4));
        for n in 0..100 {
            assert_eq!(div_dither(n, 7, &mut a), div_dither(n, 7, &mut b));
        }
    }

    #[test]
    fn sin_cos_hit_the_cardinal_angles_exactly() {
        assert_eq!(sin_cos(0), (0, TRIG_ONE));
        assert_eq!(sin_cos(9_000), (TRIG_ONE, 0));
        assert_eq!(sin_cos(18_000), (0, -TRIG_ONE));
        assert_eq!(sin_cos(27_000), (-TRIG_ONE, 0));
        assert_eq!(sin_cos(36_000), sin_cos(0), "a full turn wraps");
        assert_eq!(sin_cos(-9_000), (-TRIG_ONE, 0), "negative angles wrap");
    }

    #[test]
    fn sin_cos_are_accurate_and_on_the_unit_circle() {
        // sin 30° = 0.5 and cos 60° = 0.5, to within a few units in 2^30.
        let half = TRIG_ONE / 2;
        assert!((sin_cos(3_000).0 - half).abs() <= 2);
        assert!((sin_cos(6_000).1 - half).abs() <= 2);
        // sin 45° = cos 45° = √2/2.
        let (s, c) = sin_cos(4_500);
        assert!((s - 759_250_125).abs() <= 2 && (c - 759_250_125).abs() <= 2);
        // sin² + cos² = 1 everywhere, to within rounding.
        let one = (TRIG_ONE as i128) * (TRIG_ONE as i128);
        for a in (0..36_000).step_by(137) {
            let (s, c) = sin_cos(a);
            let r = (s as i128) * (s as i128) + (c as i128) * (c as i128);
            assert!((r - one).abs() < (TRIG_ONE as i128) * 8, "angle {a}");
        }
    }
}
