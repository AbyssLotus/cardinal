//! The calendar and the lights of the sky (Vol. III Ch. 1 §1.10, *Time and Change*: "sunrise,
//! sunset, seasons, tides"; Amendment A-34).
//!
//! Every quantity here is a pure function of simulated time and of where on the world one stands
//! (its latitude), so it is the same at any tick length (Amendment A-1) and the same on every
//! platform (fixed-point trigonometry, [`kernel::fixed::sin_cos`]).
//!
//! - **The year.** A world may declare one: its length in days, and how many days after the
//!   spring equinox it begins. The sun's declination — how far north of the equator it stands —
//!   follows the year as a sine of the world's axial tilt.
//! - **The sun's height.** `sin(height) = sin φ · sin δ + cos φ · cos δ · cos h`, for latitude
//!   `φ`, declination `δ`, and the hour angle `h` (zero at noon). Its light is the world's peak
//!   times that height wherever the sun is up, so days lengthen toward midsummer and noon stands
//!   lower in winter. At the equator, at an equinox, it is the plain day the engine always had.
//! - **The seasons' warmth.** A swing of the world's declared size, warmest the declared lag after
//!   midsummer, coldest the same lag after midwinter; reversed in the southern hemisphere.
//! - **The moon.** A month of declared length. Its light is the world's moonlight at full, by its
//!   phase and its height: it rises later each night, a full moon stands opposite the sun —
//!   high in the winter's night sky — and a new moon gives none.

use kernel::fixed::{sin_cos, FULL_TURN, TRIG_ONE};

/// The world's calendar and sky, all world rules (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Calendar {
    /// The length of a day, in milliseconds of simulated time.
    pub day_ms: u64,
    /// The length of a year, in days. Zero: the world has no year — no seasons, an equinox sun.
    pub year_days: u64,
    /// How many days after the spring equinox the world begins.
    pub start_day: u64,
    /// The world's axial tilt, in hundredths of a degree: how far north and south of the equator
    /// the sun travels over a year.
    pub tilt: i64,
    /// The latitude of every place that declares none, in hundredths of a degree north (negative
    /// is south).
    pub latitude: i64,
    /// Half the difference between midsummer's and midwinter's normal temperature, in
    /// centidegrees.
    pub seasonal_amplitude_centi_c: i64,
    /// How many days the warmest and coldest of the year lag midsummer and midwinter.
    pub seasonal_lag_days: u64,
    /// The length of the moon's month, in milliseconds. Zero: no moon.
    pub lunar_month_ms: u64,
    /// The moon's phase when the world begins, in hundredths of a percent of its month: 0 new,
    /// 5000 full.
    pub moon_phase_at_start: i64,
    /// The light of a full moon overhead, in hundredths of a percent of full daylight.
    pub moonlight_peak: i64,
}

/// `TRIG_ONE` as an `i128`, for products of two scaled values.
const ONE: i128 = TRIG_ONE as i128;

impl Calendar {
    /// How far through its year the world is at `at_ms`, as an angle in hundredths of a degree
    /// from the spring equinox — `None` if it has no year.
    pub fn year_angle(&self, at_ms: u64) -> Option<i64> {
        if self.year_days == 0 || self.day_ms == 0 {
            return None;
        }
        let year = self.year_days as i128 * self.day_ms as i128;
        let since = self.start_day as i128 * self.day_ms as i128 + at_ms as i128;
        Some((since.rem_euclid(year) * FULL_TURN as i128 / year) as i64)
    }

    /// The sun's declination at `at_ms`, in hundredths of a degree north.
    pub fn declination(&self, at_ms: u64) -> i64 {
        self.year_angle(at_ms).map_or(0, |a| {
            (self.tilt as i128 * sin_cos(a).0 as i128 / ONE) as i64
        })
    }

    /// How far through its day the world is at `at_ms`, as an angle: 0 at midnight, half a turn
    /// at noon.
    fn day_angle(&self, at_ms: u64) -> i64 {
        let day = self.day_ms.max(1) as i128;
        ((at_ms as i128).rem_euclid(day) * FULL_TURN as i128 / day) as i64
    }

    /// The sine of the height above the horizon, scaled by `TRIG_ONE`, of a light at
    /// `declination` whose day stands at `day_angle` (noon half a turn), seen from `latitude`.
    fn height(latitude: i64, declination: i64, day_angle: i64) -> i64 {
        let (sin_lat, cos_lat) = sin_cos(latitude);
        let (sin_dec, cos_dec) = sin_cos(declination);
        // The hour angle is zero at noon: cos h = −cos(day angle).
        let cos_hour = -(sin_cos(day_angle).1 as i128);
        let h = (sin_lat as i128 * sin_dec as i128
            + cos_lat as i128 * cos_dec as i128 / ONE * cos_hour)
            / ONE;
        h as i64
    }

    /// The sine of the sun's height at `at_ms`, seen from `latitude`, scaled by `TRIG_ONE`:
    /// negative while it is below the horizon.
    pub fn sun_height(&self, at_ms: u64, latitude: i64) -> i64 {
        Self::height(latitude, self.declination(at_ms), self.day_angle(at_ms))
    }

    /// The moon's phase at `at_ms`, in hundredths of a percent of its month (0 new, 5000 full) —
    /// `None` if the world has no moon.
    pub fn moon_phase(&self, at_ms: u64) -> Option<i64> {
        if self.lunar_month_ms == 0 {
            return None;
        }
        let month = self.lunar_month_ms as i128;
        let since = self.moon_phase_at_start as i128 * month / 10_000 + at_ms as i128;
        Some((since.rem_euclid(month) * 10_000 / month) as i64)
    }

    /// The moon's light at `at_ms`, seen from `latitude`, in hundredths of a percent of full
    /// daylight: its full light, times how much of it is lit, times its height.
    pub fn moonlight(&self, at_ms: u64, latitude: i64) -> i64 {
        let Some(phase) = self.moon_phase(at_ms) else {
            return 0;
        };
        let turn = phase * FULL_TURN / 10_000;
        let cos_turn = sin_cos(turn).1 as i128;
        // How much of its face is lit: none at new, all at full.
        let lit = (ONE - cos_turn) / 2;
        // It lags the sun by its phase, and stands opposite it in the sky at full.
        let declination = (self.declination(at_ms) as i128 * cos_turn / ONE) as i64;
        let height = Self::height(latitude, declination, self.day_angle(at_ms) - turn).max(0);
        (self.moonlight_peak.max(0) as i128 * lit / ONE * height as i128 / ONE) as i64
    }

    /// Open-sky light at `at_ms`, seen from `latitude`, in hundredths of a percent: the sun's,
    /// `peak` overhead, and the moon's; never past `peak`.
    pub fn sky_light(&self, at_ms: u64, latitude: i64, peak: i64) -> i64 {
        let sun = self.sun_height(at_ms, latitude).max(0) as i128;
        let sunlight = (peak as i128 * sun / ONE) as i64;
        (sunlight + self.moonlight(at_ms, latitude)).min(peak)
    }

    /// The seasons' share of the temperature at `at_ms`, at `latitude`, in centidegrees: the
    /// declared swing, warmest the lag after midsummer; reversed south of the equator. Zero in a
    /// world with no year.
    pub fn seasonal(&self, at_ms: u64, latitude: i64) -> i64 {
        let Some(angle) = self.year_angle(at_ms) else {
            return 0;
        };
        let lag =
            (self.seasonal_lag_days as i128 * FULL_TURN as i128 / self.year_days as i128) as i64;
        // Midsummer in the north is a quarter-year after the spring equinox.
        let warmest = FULL_TURN / 4 + lag;
        let swing = (self.seasonal_amplitude_centi_c as i128 * sin_cos(angle - warmest).1 as i128
            / ONE) as i64;
        if latitude < 0 {
            -swing
        } else {
            swing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86_400_000;

    fn earth(start_day: u64) -> Calendar {
        Calendar {
            day_ms: DAY,
            year_days: 365,
            start_day,
            tilt: 2_344,
            latitude: 5_200,
            seasonal_amplitude_centi_c: 700,
            seasonal_lag_days: 30,
            lunar_month_ms: 2_551_443_000,
            moon_phase_at_start: 0,
            moonlight_peak: 60,
        }
    }

    /// The angle whose sine is `s` (scaled by `TRIG_ONE`), to the hundredth of a degree.
    fn asin(s: i64) -> i64 {
        (0..=9_000)
            .min_by_key(|a| (sin_cos(*a).0 - s).abs())
            .unwrap()
    }

    #[test]
    fn with_no_year_and_on_the_equator_the_sun_is_the_old_plain_day() {
        let plain = Calendar {
            year_days: 0,
            latitude: 0,
            lunar_month_ms: 0,
            ..earth(0)
        };
        for hour in 0..24 {
            let at = hour * DAY / 24;
            let angle = (at as i128 * FULL_TURN as i128 / DAY as i128) as i64;
            let old = (-sin_cos(angle).1).max(0) as i128 * 10_000 / ONE;
            assert_eq!(plain.sky_light(at, 0, 10_000) as i128, old, "hour {hour}");
        }
    }

    #[test]
    fn the_sun_stands_high_at_midsummer_and_low_at_midwinter() {
        // At 52° north, noon's sun is 90 − 52 ± 23.44 degrees high at the solstices.
        let noon = DAY / 2;
        let summer = earth(91).sun_height(noon, 5_200);
        let winter = earth(274).sun_height(noon, 5_200);
        assert!((asin(summer) - 6_144).abs() <= 10, "{}", asin(summer));
        assert!((asin(winter) - 1_456).abs() <= 10, "{}", asin(winter));
        // South of the equator, the other way round.
        assert!(earth(91).sun_height(noon, -5_200) < earth(274).sun_height(noon, -5_200));
    }

    #[test]
    fn days_are_long_in_summer_and_short_in_winter() {
        let daylight_hours = |cal: Calendar| {
            (0..24 * 60)
                .filter(|m| cal.sun_height(m * 60_000, 5_200) > 0)
                .count() as i64
                / 60
        };
        assert_eq!(daylight_hours(earth(0)), 12, "an equinox");
        assert_eq!(daylight_hours(earth(91)), 16, "midsummer");
        assert_eq!(daylight_hours(earth(274)), 7, "midwinter");
    }

    #[test]
    fn the_warmest_of_the_year_lags_midsummer() {
        // A swing wide enough that a day's difference shows in whole centidegrees.
        let cal = Calendar {
            seasonal_amplitude_centi_c: 1_000_000,
            ..earth(0)
        };
        let warmth: Vec<i64> = (0..365).map(|d| cal.seasonal(d * DAY, 5_200)).collect();
        let warmest = warmth
            .iter()
            .enumerate()
            .max_by_key(|(_, w)| **w)
            .unwrap()
            .0;
        let coldest = warmth
            .iter()
            .enumerate()
            .min_by_key(|(_, w)| **w)
            .unwrap()
            .0;
        assert_eq!(warmest, 121, "a month after midsummer, day 91");
        assert!((303..=304).contains(&coldest), "{coldest}");
        assert!(*warmth.iter().max().unwrap() >= 999_990);
        // South of the equator, midsummer's warmth is midwinter's.
        assert_eq!(cal.seasonal(121 * DAY, -5_200), -warmth[121]);
        // A world begun the lag after the equinox begins at its year's mean.
        assert_eq!(earth(30).seasonal(0, 5_200), 0);
    }

    #[test]
    fn the_full_moon_lights_the_night_and_the_new_moon_does_not() {
        let cal = earth(0);
        let midnight = |day: u64| day * DAY;
        assert_eq!(cal.moonlight(midnight(0), 5_200), 0, "new moon");
        // Fifteen days on, near full, the moon stands south at midnight.
        let full = cal.moonlight(midnight(15), 5_200);
        assert!(full > 20 && full <= 60, "{full}");
        // And it has set by noon.
        assert_eq!(cal.moonlight(midnight(15) + DAY / 2, 5_200), 0);
    }
}
