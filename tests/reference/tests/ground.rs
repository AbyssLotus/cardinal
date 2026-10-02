//! Ground in Ashford (Vol. III Ch. 1 §1.10–1.11; Amendments A-4, A-5): the Hill's terrain
//! between its samples, how steep it is, how high everyone stands above the ground beneath
//! them, and how far one would drop through each kind of opening.

use physical::schema::PORTAL_DANGER;
use physical::space::{height_above_datum, height_above_ground};
use physical::terrain::{ground, slope_percent};
use reference::id::*;
use reference::{e, City};

#[test]
fn the_hill_blends_between_its_samples() {
    let city = City::new();
    let s = city.store();
    assert_eq!(ground(s, e(HILL), 0, 0), 0);
    assert_eq!(
        ground(s, e(HILL), 500, 1_000),
        50,
        "halfway up the first rise"
    );
    assert_eq!(
        ground(s, e(HILL), 2_000, 1_000),
        300,
        "where the cottage stands"
    );
    assert_eq!(
        ground(s, e(HILL), 3_500, 1_000),
        900,
        "halfway up the cliff"
    );
    assert_eq!(ground(s, e(HILL), 4_000, 2_000), 1_500, "the ridgeline");
    assert_eq!(ground(s, e(HILL), 4_000, 4_500), 300, "the pass");
    assert_eq!(slope_percent(s, e(HILL), 500, 500), Some(10));
    assert_eq!(
        slope_percent(s, e(HILL), 3_500, 1_000),
        Some(120),
        "too steep to walk"
    );
    assert_eq!(slope_percent(s, e(HILL), 3_500, 4_500), Some(0));
}

#[test]
fn heights_are_measured_from_the_ground_beneath() {
    let city = City::new();
    let s = city.store();
    // The villager at the foot of the Hill stands on it, half a metre above the datum.
    assert_eq!(height_above_ground(s, e(VILLAGER)), 0);
    assert_eq!(height_above_datum(s, e(VILLAGER)), 50);
    // Lena stands on the ridgeline, 15 m up — on the ground.
    assert_eq!(height_above_ground(s, e(LENA)), 0);
    assert_eq!(height_above_datum(s, e(LENA)), 1_500);
    // Nell, on top of her boulder, is 3 m above the slope.
    assert_eq!(height_above_ground(s, e(NELL)), 300);
    // In Old Town, which has no terrain, the ground is the datum: a storey up, a cellar down.
    assert_eq!(height_above_ground(s, e(ALICE)), 300);
    assert_eq!(height_above_ground(s, e(RACCOON)), -300);
}

#[test]
fn each_opening_is_as_dangerous_as_its_drop() {
    let mut city = City::new();
    city.run(1);
    let danger = |portal| city.int(portal, PORTAL_DANGER).unwrap();
    // 15 danger points per centimetre of drop (1500 per metre), measured where one lands.
    assert_eq!(danger(BEDROOM_WINDOW), 5_700, "3.8 m down to the yard");
    assert_eq!(danger(WINDOW_OUT), 450, "30 cm down from the low sill");
    assert_eq!(danger(DOOR_OUT), 0);
    assert_eq!(danger(STAIRS_UP), 0, "stairs land you on a floor");
    assert_eq!(danger(STAIRS_DOWN), 0);
    assert_eq!(
        danger(GALLERY_WINDOW),
        6_000,
        "4 m down to the manor grounds"
    );
    assert_eq!(danger(MANOR_DOOR_IN), 0, "pinned harmless by the world");
    assert_eq!(
        danger(COTTAGE_DOOR_IN),
        0,
        "a ground-floor door, however high the hill"
    );
    assert_eq!(danger(COTTAGE_DOOR_OUT), 0);
}
