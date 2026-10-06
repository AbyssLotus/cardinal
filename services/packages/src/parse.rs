//! A minimal, dependency-free parser for the world-file format (Vol. IV Ch. 1 §1.2).
//!
//! The package layout is "conceptual, not prescriptive" (Vol. IV Ch. 1 §1.2), so this is one
//! concrete surface: a small sectioned `key = value` text format, parsed with only the
//! standard library so the engine keeps its zero-dependency, offline build. Packages remain
//! pure data — this reads declarations, it never executes them (Vol. IV Ch. 1, invariant 6).

use crate::model::{
    AdjacencySpec, BodySpec, ClockRules, ContainmentSpec, DepositSpec, ExposureSpec, FacingSpec,
    Flag, FlagSpec, InformationRules, JobSpec, JobWork, LivingRules, MadeOfSpec, Manifest,
    MaterialProperty, MaterialSpec, MindsRules, MotionSpec, NeedKindSpec, OrganismSpec,
    PhysicalRules, PortalDangerSpec, PortalSpec, PositionSpec, RecipeSpec, RegionMembershipSpec,
    RegionSpec, ResourcesRules, SocietyRules, TerrainSpec, TravelSpec, WorldPackage,
};
use crate::version::{EngineReq, Version};
use std::fmt;
use std::str::FromStr;

/// A failure to parse a world file, with a 1-based line number and a reason.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseError {
    /// The 1-based line number the error was found on (0 if not line-specific).
    pub line: usize,
    /// A human-readable explanation.
    pub reason: String,
}

impl ParseError {
    fn at(line: usize, reason: impl Into<String>) -> Self {
        Self {
            line,
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "world-file parse error (line {}): {}",
            self.line, self.reason
        )
    }
}

/// Parse a world file into a [`WorldPackage`].
///
/// Recognised sections: `[manifest]`, `[clock]` (`tick_seconds` or `tick_ms`, and
/// `day_seconds`), `[rules.physical]`, `[rules.living]`, `[regions]`
/// (`region_id = temperature[, elevation]`), `[organisms]`
/// (`organism_id = region_id, body_heat`), `[containment]` (`child_id = parent_id`),
/// `[adjacency]`, `[exposure]`, `[positions]`, `[portals]`, `[portal_danger]`, `[materials]`
/// (`material_id = property:value, …`), `[made_of]` (`object_id = material_id[, …]`),
/// `[in_region]` (`location_id = region_id[, …]`), `[bodies]`
/// (`entity_id = half_width, half_depth, height`), `[facing]` (`entity_id = degrees`), and
/// `[motion]` (`entity_id = x, y, z, seconds`), `[flags]` (`entity_id = flag[, …]`, flags
/// `solid`, `opaque`, `enclosed`, `mobile`, `closed`), `[portal_pairs]` (`portal = portal`),
/// `[terrain]` (`region_id = spacing, columns, h h h …` row-major), `[travel]`
/// (`entity_id = target_id, speed`), and `[places]` (`place_id = container_id` or `none`).
/// Blank lines and `#` comments are ignored. A missing required field is an error — the
/// loader never fabricates defaults (Vol. IV Ch. 2).
pub fn parse_world(text: &str) -> Result<WorldPackage, ParseError> {
    let mut section = String::new();
    let mut id: Option<String> = None;
    let mut version: Option<Version> = None;
    let mut engine: Option<EngineReq> = None;
    let mut domains: Option<Vec<String>> = None;
    let mut tick_ms: Option<u64> = None;
    let mut day_seconds: Option<u64> = None;
    let mut environment_step_seconds: Option<u64> = None;
    let mut amplitude: Option<i64> = None;
    let mut temperature_variability: Option<i64> = None;
    let mut weather_persistence_seconds: Option<u64> = None;
    let mut illumination_peak: Option<i64> = None;
    let mut humidity_baseline: Option<i64> = None;
    let mut humidity_variability: Option<i64> = None;
    let mut pressure_sea_level: Option<i64> = None;
    let mut pressure_elevation_factor: Option<i64> = None;
    let mut pressure_variability: Option<i64> = None;
    let mut wind_gradient_divisor: Option<i64> = None;
    let mut fall_danger_per_meter: Option<i64> = None;
    let mut thermal_mass_reference: Option<i64> = None;
    let mut gravity_cm_s2: Option<i64> = None;
    let mut step_height_cm: Option<i64> = None;
    let mut max_slope_percent: Option<i64> = None;
    let mut nav_cell_cm: Option<i64> = None;
    let mut reach_cm: Option<i64> = None;
    let mut indoor_coupling_seconds: Option<u64> = None;
    let mut sight_step_seconds: Option<u64> = None;
    let mut sight_min_illumination: Option<i64> = None;
    let mut carry_limit_kg: Option<i64> = None;
    let mut perception_step_seconds: Option<u64> = None;
    let mut warmth_resolution: Option<i64> = None;
    let mut senses: Vec<(u64, i64)> = Vec::new();
    let mut knows: Vec<(u64, Vec<u64>)> = Vec::new();
    let mut think_step_seconds: Option<u64> = None;
    let mut cold_below: Option<i64> = None;
    let mut trust_half_age_seconds: Option<u64> = None;
    let mut hop_cost: Option<i64> = None;
    let mut routine_value: Option<i64> = None;
    let mut switch_margin: Option<i64> = None;
    let mut minds: Vec<(u64, i64)> = Vec::new();
    let mut routines: Vec<(u64, i64, i64, u64)> = Vec::new();
    let mut metabolism_step_seconds: Option<u64> = None;
    let mut set_point: Option<i64> = None;
    let mut warm_response_seconds: Option<u64> = None;
    let mut cold_response_seconds: Option<u64> = None;
    let mut tire_per_hour: Option<i64> = None;
    let mut rest_per_hour: Option<i64> = None;
    let mut hypothermia_below: Option<i64> = None;
    let mut cold_harm: Option<i64> = None;
    let mut safe_fall: Option<i64> = None;
    let mut fall_harm: Option<i64> = None;
    let mut heal_per_hour: Option<i64> = None;
    let mut need_resolution: Option<i64> = None;
    let mut tired_above: Option<i64> = None;
    let mut rested_below: Option<i64> = None;
    let mut fatigue: Vec<(u64, i64)> = Vec::new();
    let mut affection_step: Option<u64> = None;
    let mut affection_per_hour: Option<i64> = None;
    let mut affection_fade: Option<i64> = None;
    let mut need_above: Option<i64> = None;
    let mut need_weight: Option<i64> = None;
    let mut hungry_above: Option<i64> = None;
    let mut work_value: Option<i64> = None;
    let mut tired_margin: Option<i64> = None;
    let mut sleep_hours: Option<(i64, i64)> = None;
    let mut courtship_step: Option<u64> = None;
    let mut bond_above: Option<i64> = None;
    let mut part_below: Option<i64> = None;
    let mut temperament: Vec<(u64, [i64; 3])> = Vec::new();
    let mut need_kinds: Vec<NeedKindSpec> = Vec::new();
    let mut dependence: Vec<(u64, u64, i64)> = Vec::new();
    let mut deposits: Vec<DepositSpec> = Vec::new();
    let mut recipes: Vec<RecipeSpec> = Vec::new();
    let mut jobs: Vec<JobSpec> = Vec::new();
    let mut roles: Vec<(u64, u64)> = Vec::new();
    let mut owners: Vec<(u64, u64)> = Vec::new();
    let mut curiosity: Vec<(u64, i64)> = Vec::new();
    let mut likes: Vec<(u64, u64, i64)> = Vec::new();
    let mut home: Vec<(u64, u64)> = Vec::new();
    let mut regrow_step: Option<u64> = None;
    let mut dependence_fade: Option<i64> = None;
    let mut hunger_per_hour: Option<i64> = None;
    let mut starving_above: Option<i64> = None;
    let mut starving_harm: Option<i64> = None;
    let mut hunger: Vec<(u64, i64)> = Vec::new();
    let mut regions: Vec<RegionSpec> = Vec::new();
    let mut organisms: Vec<OrganismSpec> = Vec::new();
    let mut containment: Vec<ContainmentSpec> = Vec::new();
    let mut adjacency: Vec<AdjacencySpec> = Vec::new();
    let mut exposure: Vec<ExposureSpec> = Vec::new();
    let mut positions: Vec<PositionSpec> = Vec::new();
    let mut portals: Vec<PortalSpec> = Vec::new();
    let mut portal_danger: Vec<PortalDangerSpec> = Vec::new();
    let mut materials: Vec<MaterialSpec> = Vec::new();
    let mut made_of: Vec<MadeOfSpec> = Vec::new();
    let mut in_region: Vec<RegionMembershipSpec> = Vec::new();
    let mut bodies: Vec<BodySpec> = Vec::new();
    let mut facing: Vec<FacingSpec> = Vec::new();
    let mut motion: Vec<MotionSpec> = Vec::new();
    let mut flags: Vec<FlagSpec> = Vec::new();
    let mut portal_pairs: Vec<(u64, u64)> = Vec::new();
    let mut terrain: Vec<TerrainSpec> = Vec::new();
    let mut travel: Vec<TravelSpec> = Vec::new();
    let mut places: Vec<(u64, Option<u64>)> = Vec::new();

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = name.trim().to_string();
            continue;
        }
        let (key, value) = split_kv(line, line_no)?;
        match section.as_str() {
            "manifest" => match key {
                "id" => id = Some(value.to_string()),
                "version" => {
                    version = Some(
                        Version::parse(value)
                            .map_err(|e| ParseError::at(line_no, e.to_string()))?,
                    )
                }
                "engine" => {
                    engine = Some(
                        EngineReq::parse(value)
                            .map_err(|e| ParseError::at(line_no, e.to_string()))?,
                    )
                }
                "domains" => {
                    domains = Some(
                        value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                    )
                }
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown manifest key {other:?}"),
                    ))
                }
            },
            "clock" => match key {
                // The tick may be declared in whole seconds or in milliseconds — one or the
                // other — so an hour-long tick and a tenth-of-a-second tick both read naturally.
                "tick_seconds" | "tick_ms" => {
                    if tick_ms.is_some() {
                        return Err(ParseError::at(
                            line_no,
                            "declare the tick length once, as tick_seconds or tick_ms",
                        ));
                    }
                    let n: u64 = parse_num(value, line_no)?;
                    let ms = if key == "tick_seconds" {
                        n.checked_mul(1000)
                            .ok_or_else(|| ParseError::at(line_no, "tick length overflows"))?
                    } else {
                        n
                    };
                    if ms == 0 {
                        return Err(ParseError::at(
                            line_no,
                            "a tick must last some time (zero length)",
                        ));
                    }
                    tick_ms = Some(ms);
                }
                "day_seconds" => day_seconds = Some(parse_num(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown clock key {other:?}"),
                    ))
                }
            },
            "rules.physical" => match key {
                "environment_step_seconds" => {
                    environment_step_seconds = Some(parse_num(value, line_no)?)
                }
                "diurnal_amplitude_centi_c" => amplitude = Some(parse_num(value, line_no)?),
                "temperature_variability_centi_c" => {
                    temperature_variability = Some(parse_num(value, line_no)?)
                }
                "weather_persistence_seconds" => {
                    weather_persistence_seconds = Some(parse_num(value, line_no)?)
                }
                "illumination_peak" => illumination_peak = Some(parse_num(value, line_no)?),
                "humidity_baseline" => humidity_baseline = Some(parse_num(value, line_no)?),
                "humidity_variability" => humidity_variability = Some(parse_num(value, line_no)?),
                "pressure_sea_level" => pressure_sea_level = Some(parse_num(value, line_no)?),
                "pressure_elevation_factor" => {
                    pressure_elevation_factor = Some(parse_num(value, line_no)?)
                }
                "pressure_variability" => pressure_variability = Some(parse_num(value, line_no)?),
                "wind_gradient_divisor" => wind_gradient_divisor = Some(parse_num(value, line_no)?),
                "fall_danger_per_meter" => fall_danger_per_meter = Some(parse_num(value, line_no)?),
                "thermal_mass_reference" => {
                    thermal_mass_reference = Some(parse_num(value, line_no)?)
                }
                "gravity_cm_s2" => gravity_cm_s2 = Some(parse_num(value, line_no)?),
                "step_height_cm" => step_height_cm = Some(parse_num(value, line_no)?),
                "max_slope_percent" => max_slope_percent = Some(parse_num(value, line_no)?),
                "nav_cell_cm" => nav_cell_cm = Some(parse_num(value, line_no)?),
                "reach_cm" => reach_cm = Some(parse_num(value, line_no)?),
                "indoor_coupling_seconds" => {
                    indoor_coupling_seconds = Some(parse_num(value, line_no)?)
                }
                "sight_step_seconds" => sight_step_seconds = Some(parse_num(value, line_no)?),
                "sight_min_illumination" => {
                    sight_min_illumination = Some(parse_num(value, line_no)?)
                }
                "carry_limit_kg" => carry_limit_kg = Some(parse_num(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown physical rule {other:?}"),
                    ))
                }
            },
            "rules.living" => match key {
                "metabolism_step_seconds" => {
                    metabolism_step_seconds = Some(parse_num(value, line_no)?)
                }
                "set_point_centi_c" => set_point = Some(parse_num(value, line_no)?),
                "warm_response_seconds" => warm_response_seconds = Some(parse_num(value, line_no)?),
                "cold_response_seconds" => cold_response_seconds = Some(parse_num(value, line_no)?),
                "tire_per_hour" => tire_per_hour = Some(parse_num(value, line_no)?),
                "rest_per_hour" => rest_per_hour = Some(parse_num(value, line_no)?),
                "hypothermia_below_centi_c" => hypothermia_below = Some(parse_num(value, line_no)?),
                "cold_harm_per_degree_hour" => cold_harm = Some(parse_num(value, line_no)?),
                "safe_fall_cm" => safe_fall = Some(parse_num(value, line_no)?),
                "fall_harm_per_metre" => fall_harm = Some(parse_num(value, line_no)?),
                "heal_per_hour" => heal_per_hour = Some(parse_num(value, line_no)?),
                "dependence_fade_per_day" => dependence_fade = Some(parse_num(value, line_no)?),
                "hunger_per_hour" => hunger_per_hour = Some(parse_num(value, line_no)?),
                "starving_above" => starving_above = Some(parse_num(value, line_no)?),
                "starving_harm_per_hour" => starving_harm = Some(parse_num(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown living rule {other:?}"),
                    ))
                }
            },
            "rules.information" => match key {
                "perception_step_seconds" => {
                    perception_step_seconds = Some(parse_num(value, line_no)?)
                }
                "warmth_resolution_centi_c" => warmth_resolution = Some(parse_num(value, line_no)?),
                "need_resolution" => need_resolution = Some(parse_num(value, line_no)?),
                "affection_step_seconds" => affection_step = Some(parse_num(value, line_no)?),
                "affection_per_hour" => affection_per_hour = Some(parse_num(value, line_no)?),
                "affection_fade_per_day" => affection_fade = Some(parse_num(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown information rule {other:?}"),
                    ))
                }
            },
            "rules.minds" => match key {
                "think_step_seconds" => think_step_seconds = Some(parse_num(value, line_no)?),
                "cold_below_centi_c" => cold_below = Some(parse_num(value, line_no)?),
                "trust_half_age_seconds" => {
                    trust_half_age_seconds = Some(parse_num(value, line_no)?)
                }
                "hop_cost" => hop_cost = Some(parse_num(value, line_no)?),
                "routine_value" => routine_value = Some(parse_num(value, line_no)?),
                "switch_margin" => switch_margin = Some(parse_num(value, line_no)?),
                "tired_above" => tired_above = Some(parse_num(value, line_no)?),
                "rested_below" => rested_below = Some(parse_num(value, line_no)?),
                "need_above" => need_above = Some(parse_num(value, line_no)?),
                "need_weight" => need_weight = Some(parse_num(value, line_no)?),
                "hungry_above" => hungry_above = Some(parse_num(value, line_no)?),
                "work_value" => work_value = Some(parse_num(value, line_no)?),
                "tired_margin" => tired_margin = Some(parse_num(value, line_no)?),
                "sleep_hours" => sleep_hours = Some(parse_hours(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown minds rule {other:?}"),
                    ))
                }
            },
            "rules.society" => match key {
                "courtship_step_seconds" => courtship_step = Some(parse_num(value, line_no)?),
                "bond_above" => bond_above = Some(parse_num(value, line_no)?),
                "part_below" => part_below = Some(parse_num(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown society rule {other:?}"),
                    ))
                }
            },
            "temperament" => {
                let mind: u64 = parse_num(key, line_no)?;
                temperament.push((mind, parse_ints::<3>(value, line_no, "a, b, c")?));
            }
            "rules.resources" => match key {
                "regrow_step_seconds" => regrow_step = Some(parse_num(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown resources rule {other:?}"),
                    ))
                }
            },
            "deposits" => {
                let id: u64 = parse_num(key, line_no)?;
                deposits.push(parse_deposit(id, value, line_no)?);
            }
            "recipes" => {
                let id: u64 = parse_num(key, line_no)?;
                recipes.push(parse_recipe(id, value, line_no)?);
            }
            "jobs" => {
                let id: u64 = parse_num(key, line_no)?;
                jobs.push(parse_job(id, value, line_no)?);
            }
            "roles" => roles.push((parse_num(key, line_no)?, parse_num(value, line_no)?)),
            "owners" => owners.push((parse_num(key, line_no)?, parse_num(value, line_no)?)),
            "curiosity" => curiosity.push((parse_num(key, line_no)?, parse_num(value, line_no)?)),
            "home" => home.push((parse_num(key, line_no)?, parse_num(value, line_no)?)),
            "likes" => {
                // mind = material:worth, material:worth
                let mind: u64 = parse_num(key, line_no)?;
                for part in value.split(',') {
                    let (m, w) = part.split_once(':').ok_or_else(|| {
                        ParseError::at(line_no, format!("expected material:worth, got {part:?}"))
                    })?;
                    likes.push((mind, parse_num(m, line_no)?, parse_num(w, line_no)?));
                }
            }
            "hunger" => {
                let organism: u64 = parse_num(key, line_no)?;
                hunger.push((organism, parse_num(value, line_no)?));
            }
            "dependence" => {
                let organism: u64 = parse_num(key, line_no)?;
                let [material, level] = parse_ints::<2>(value, line_no, "material, level")?;
                dependence.push((organism, material as u64, level));
            }
            "need_kinds" => {
                let id: u64 = parse_num(key, line_no)?;
                need_kinds.push(parse_need_kind(id, value, line_no)?);
            }
            "fatigue" => {
                let organism: u64 = parse_num(key, line_no)?;
                fatigue.push((organism, parse_num(value, line_no)?));
            }
            "minds" => {
                let mind: u64 = parse_num(key, line_no)?;
                minds.push((mind, parse_num(value, line_no)?));
            }
            "routines" => {
                let mind: u64 = parse_num(key, line_no)?;
                let [from, to, target] =
                    parse_ints::<3>(value, line_no, "from_hour, to_hour, where")?;
                if !(0..=24).contains(&from) || !(0..=24).contains(&to) || target < 0 {
                    return Err(ParseError::at(
                        line_no,
                        "a routine is from_hour, to_hour (0-24), where",
                    ));
                }
                routines.push((mind, from, to, target as u64));
            }
            "senses" => {
                let organism: u64 = parse_num(key, line_no)?;
                senses.push((organism, parse_num(value, line_no)?));
            }
            "knows" => {
                let mind: u64 = parse_num(key, line_no)?;
                let things = value
                    .split(',')
                    .map(|t| parse_num(t.trim(), line_no))
                    .collect::<Result<Vec<u64>, _>>()?;
                knows.push((mind, things));
            }
            "regions" => {
                let region_id: u64 = parse_num(key, line_no)?;
                let (temp, elevation) = parse_region_values(value, line_no)?;
                regions.push(RegionSpec {
                    id: region_id,
                    temperature_centi_c: temp,
                    elevation,
                });
            }
            "organisms" => {
                let organism_id: u64 = parse_num(key, line_no)?;
                let (region_id, body_heat) = split_pair(value, line_no)?;
                organisms.push(OrganismSpec {
                    id: organism_id,
                    region_id,
                    body_heat_centi_c: body_heat,
                });
            }
            "containment" => {
                let child_id: u64 = parse_num(key, line_no)?;
                let parent_id: u64 = parse_num(value, line_no)?;
                containment.push(ContainmentSpec {
                    child_id,
                    parent_id,
                });
            }
            "adjacency" => {
                let a: u64 = parse_num(key, line_no)?;
                let b: u64 = parse_num(value, line_no)?;
                adjacency.push(AdjacencySpec { a, b });
            }
            "exposure" => {
                let region_id: u64 = parse_num(key, line_no)?;
                let exp: i64 = parse_num(value, line_no)?;
                exposure.push(ExposureSpec {
                    region_id,
                    exposure: exp,
                });
            }
            "positions" => {
                let entity_id: u64 = parse_num(key, line_no)?;
                let (x, y, z) = parse_position_values(value, line_no)?;
                positions.push(PositionSpec { entity_id, x, y, z });
            }
            "portals" => {
                let portal_id: u64 = parse_num(key, line_no)?;
                let (host_region, dest_region, x, y, z) = parse_portal_values(value, line_no)?;
                portals.push(PortalSpec {
                    portal_id,
                    host_region,
                    dest_region,
                    x,
                    y,
                    z,
                });
            }
            "portal_danger" => {
                let portal_id: u64 = parse_num(key, line_no)?;
                let danger: i64 = parse_num(value, line_no)?;
                portal_danger.push(PortalDangerSpec { portal_id, danger });
            }
            "materials" => {
                let id: u64 = parse_num(key, line_no)?;
                let properties = parse_material_properties(value, line_no)?;
                materials.push(MaterialSpec { id, properties });
            }
            "made_of" => {
                let object_id: u64 = parse_num(key, line_no)?;
                for material in value.split(',') {
                    let material_id: u64 = parse_num(material, line_no)?;
                    made_of.push(MadeOfSpec {
                        object_id,
                        material_id,
                    });
                }
            }
            "in_region" => {
                let location_id: u64 = parse_num(key, line_no)?;
                for region in value.split(',') {
                    let region_id: u64 = parse_num(region, line_no)?;
                    // Lying within oneself says nothing and would only be a typo for another
                    // id; refuse it here rather than seed a meaningless fact (the region queries
                    // would tolerate it, but a package should not carry it).
                    if region_id == location_id {
                        return Err(ParseError::at(
                            line_no,
                            format!("entity {location_id} cannot be a region of itself"),
                        ));
                    }
                    in_region.push(RegionMembershipSpec {
                        location_id,
                        region_id,
                    });
                }
            }
            "bodies" => {
                let entity_id: u64 = parse_num(key, line_no)?;
                let [half_width, half_depth, height] =
                    parse_ints::<3>(value, line_no, "half_width, half_depth, height")?;
                if half_width < 0 || half_depth < 0 || height < 0 {
                    return Err(ParseError::at(line_no, "a body's size cannot be negative"));
                }
                bodies.push(BodySpec {
                    entity_id,
                    half_width,
                    half_depth,
                    height,
                });
            }
            "facing" => {
                let entity_id: u64 = parse_num(key, line_no)?;
                facing.push(FacingSpec {
                    entity_id,
                    heading: parse_degrees(value, line_no)?,
                });
            }
            "motion" => {
                let entity_id: u64 = parse_num(key, line_no)?;
                let [x, y, z, seconds] = parse_ints::<4>(value, line_no, "x, y, z, seconds")?;
                if seconds < 0 {
                    return Err(ParseError::at(line_no, "arrival cannot be in the past"));
                }
                motion.push(MotionSpec {
                    entity_id,
                    target: [x, y, z],
                    seconds: seconds as u64,
                });
            }
            "flags" => {
                let entity_id: u64 = parse_num(key, line_no)?;
                let mut list = Vec::new();
                for name in value.split(',') {
                    list.push(match name.trim() {
                        "solid" => Flag::Solid,
                        "opaque" => Flag::Opaque,
                        "enclosed" => Flag::Enclosed,
                        "mobile" => Flag::Mobile,
                        "closed" => Flag::Closed,
                        other => {
                            return Err(ParseError::at(
                                line_no,
                                format!(
                                "unknown flag {other:?} (solid, opaque, enclosed, mobile, closed)"
                            ),
                            ))
                        }
                    });
                }
                flags.push(FlagSpec {
                    entity_id,
                    flags: list,
                });
            }
            "portal_pairs" => {
                let a: u64 = parse_num(key, line_no)?;
                let b: u64 = parse_num(value, line_no)?;
                portal_pairs.push((a, b));
            }
            "terrain" => {
                let region_id: u64 = parse_num(key, line_no)?;
                let mut parts = value.splitn(3, ',');
                let mut next = |what: &str| {
                    parts.next().ok_or_else(|| {
                        ParseError::at(
                            line_no,
                            format!("expected `spacing, columns, heights…` ({what})"),
                        )
                    })
                };
                let spacing: i64 = parse_num(next("spacing")?, line_no)?;
                let columns: usize = parse_num(next("columns")?, line_no)?;
                let heights = next("heights")?
                    .split_whitespace()
                    .map(|h| parse_num(h, line_no))
                    .collect::<Result<Vec<i64>, _>>()?;
                if spacing <= 0
                    || columns == 0
                    || heights.is_empty()
                    || heights.len() % columns != 0
                {
                    return Err(ParseError::at(
                        line_no,
                        "terrain needs a positive spacing and whole rows of heights",
                    ));
                }
                terrain.push(TerrainSpec {
                    region_id,
                    spacing,
                    columns,
                    heights,
                });
            }
            "places" => {
                let place: u64 = parse_num(key, line_no)?;
                let within = match value.trim() {
                    "none" => None,
                    v => Some(parse_num(v, line_no)?),
                };
                places.push((place, within));
            }
            "travel" => {
                let entity_id: u64 = parse_num(key, line_no)?;
                let [target, speed_cm_s] = parse_ints::<2>(value, line_no, "target, speed")?;
                if target < 0 || speed_cm_s <= 0 {
                    return Err(ParseError::at(
                        line_no,
                        "travel needs a target id and a positive speed",
                    ));
                }
                travel.push(TravelSpec {
                    entity_id,
                    target: target as u64,
                    speed_cm_s,
                });
            }
            "" => {
                return Err(ParseError::at(
                    line_no,
                    "key/value appears before any [section]",
                ))
            }
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown section {other:?}"),
                ))
            }
        }
    }

    let manifest = Manifest {
        id: require(id, "manifest.id")?,
        version: require(version, "manifest.version")?,
        engine: require(engine, "manifest.engine")?,
        domains: require(domains, "manifest.domains")?,
    };
    let clock = ClockRules {
        tick_ms: require(tick_ms, "clock.tick_seconds (or clock.tick_ms)")?,
        day_seconds: require(day_seconds, "clock.day_seconds")?,
    };
    let physical_rules = PhysicalRules {
        environment_step_seconds: require(
            environment_step_seconds,
            "rules.physical.environment_step_seconds",
        )?,
        diurnal_amplitude_centi_c: require(amplitude, "rules.physical.diurnal_amplitude_centi_c")?,
        temperature_variability_centi_c: require(
            temperature_variability,
            "rules.physical.temperature_variability_centi_c",
        )?,
        weather_persistence_seconds: require(
            weather_persistence_seconds,
            "rules.physical.weather_persistence_seconds",
        )?,
        illumination_peak: require(illumination_peak, "rules.physical.illumination_peak")?,
        humidity_baseline: require(humidity_baseline, "rules.physical.humidity_baseline")?,
        humidity_variability: require(humidity_variability, "rules.physical.humidity_variability")?,
        pressure_sea_level: require(pressure_sea_level, "rules.physical.pressure_sea_level")?,
        pressure_elevation_factor: require(
            pressure_elevation_factor,
            "rules.physical.pressure_elevation_factor",
        )?,
        pressure_variability: require(pressure_variability, "rules.physical.pressure_variability")?,
        wind_gradient_divisor: require(
            wind_gradient_divisor,
            "rules.physical.wind_gradient_divisor",
        )?,
        fall_danger_per_meter: require(
            fall_danger_per_meter,
            "rules.physical.fall_danger_per_meter",
        )?,
        thermal_mass_reference: require(
            thermal_mass_reference,
            "rules.physical.thermal_mass_reference",
        )?,
        gravity_cm_s2: require(gravity_cm_s2, "rules.physical.gravity_cm_s2")?,
        step_height_cm: require(step_height_cm, "rules.physical.step_height_cm")?,
        max_slope_percent: require(max_slope_percent, "rules.physical.max_slope_percent")?,
        nav_cell_cm: require(nav_cell_cm, "rules.physical.nav_cell_cm")?,
        reach_cm: require(reach_cm, "rules.physical.reach_cm")?,
        indoor_coupling_seconds: require(
            indoor_coupling_seconds,
            "rules.physical.indoor_coupling_seconds",
        )?,
        sight_step_seconds: require(sight_step_seconds, "rules.physical.sight_step_seconds")?,
        sight_min_illumination: require(
            sight_min_illumination,
            "rules.physical.sight_min_illumination",
        )?,
        carry_limit_kg: require(carry_limit_kg, "rules.physical.carry_limit_kg")?,
    };
    let minds_rules = match (
        think_step_seconds,
        cold_below,
        trust_half_age_seconds,
        hop_cost,
        routine_value,
        switch_margin,
    ) {
        (None, None, None, None, None, None) => None,
        _ => Some(MindsRules {
            think_step_seconds: require(think_step_seconds, "rules.minds.think_step_seconds")?,
            cold_below_centi_c: require(cold_below, "rules.minds.cold_below_centi_c")?,
            trust_half_age_seconds: require(
                trust_half_age_seconds,
                "rules.minds.trust_half_age_seconds",
            )?,
            hop_cost: require(hop_cost, "rules.minds.hop_cost")?,
            routine_value: require(routine_value, "rules.minds.routine_value")?,
            switch_margin: require(switch_margin, "rules.minds.switch_margin")?,
            tired_above: require(tired_above, "rules.minds.tired_above")?,
            rested_below: require(rested_below, "rules.minds.rested_below")?,
            need_above: require(need_above, "rules.minds.need_above")?,
            need_weight: require(need_weight, "rules.minds.need_weight")?,
            hungry_above: require(hungry_above, "rules.minds.hungry_above")?,
            work_value: require(work_value, "rules.minds.work_value")?,
            tired_margin: require(tired_margin, "rules.minds.tired_margin")?,
            sleep_hours: require(sleep_hours, "rules.minds.sleep_hours")?,
        }),
    };
    let resources_rules = regrow_step.map(|regrow_step_seconds| ResourcesRules {
        regrow_step_seconds,
    });
    let society_rules = match (courtship_step, bond_above, part_below) {
        (None, None, None) => None,
        _ => Some(SocietyRules {
            courtship_step_seconds: require(
                courtship_step,
                "rules.society.courtship_step_seconds",
            )?,
            bond_above: require(bond_above, "rules.society.bond_above")?,
            part_below: require(part_below, "rules.society.part_below")?,
        }),
    };
    let information_rules = match (perception_step_seconds, warmth_resolution, need_resolution) {
        (None, None, None) => None,
        _ => Some(InformationRules {
            perception_step_seconds: require(
                perception_step_seconds,
                "rules.information.perception_step_seconds",
            )?,
            warmth_resolution_centi_c: require(
                warmth_resolution,
                "rules.information.warmth_resolution_centi_c",
            )?,
            need_resolution: require(need_resolution, "rules.information.need_resolution")?,
            affection_step_seconds: require(
                affection_step,
                "rules.information.affection_step_seconds",
            )?,
            affection_per_hour: require(
                affection_per_hour,
                "rules.information.affection_per_hour",
            )?,
            affection_fade_per_day: require(
                affection_fade,
                "rules.information.affection_fade_per_day",
            )?,
        }),
    };
    let living_rules = match (
        metabolism_step_seconds,
        set_point,
        warm_response_seconds,
        cold_response_seconds,
    ) {
        (None, None, None, None) => None,
        _ => Some(LivingRules {
            metabolism_step_seconds: require(
                metabolism_step_seconds,
                "rules.living.metabolism_step_seconds",
            )?,
            set_point_centi_c: require(set_point, "rules.living.set_point_centi_c")?,
            warm_response_seconds: require(
                warm_response_seconds,
                "rules.living.warm_response_seconds",
            )?,
            cold_response_seconds: require(
                cold_response_seconds,
                "rules.living.cold_response_seconds",
            )?,
            tire_per_hour: require(tire_per_hour, "rules.living.tire_per_hour")?,
            rest_per_hour: require(rest_per_hour, "rules.living.rest_per_hour")?,
            hypothermia_below_centi_c: require(
                hypothermia_below,
                "rules.living.hypothermia_below_centi_c",
            )?,
            cold_harm_per_degree_hour: require(
                cold_harm,
                "rules.living.cold_harm_per_degree_hour",
            )?,
            safe_fall_cm: require(safe_fall, "rules.living.safe_fall_cm")?,
            fall_harm_per_metre: require(fall_harm, "rules.living.fall_harm_per_metre")?,
            heal_per_hour: require(heal_per_hour, "rules.living.heal_per_hour")?,
            dependence_fade_per_day: require(
                dependence_fade,
                "rules.living.dependence_fade_per_day",
            )?,
            hunger_per_hour: require(hunger_per_hour, "rules.living.hunger_per_hour")?,
            starving_above: require(starving_above, "rules.living.starving_above")?,
            starving_harm_per_hour: require(starving_harm, "rules.living.starving_harm_per_hour")?,
        }),
    };

    Ok(WorldPackage {
        manifest,
        clock,
        physical_rules,
        living_rules,
        information_rules,
        minds_rules,
        society_rules,
        resources_rules,
        regions,
        organisms,
        containment,
        adjacency,
        exposure,
        positions,
        portals,
        portal_danger,
        materials,
        made_of,
        in_region,
        bodies,
        facing,
        motion,
        flags,
        portal_pairs,
        terrain,
        travel,
        places,
        senses,
        knows,
        minds,
        temperament,
        need_kinds,
        deposits,
        recipes,
        jobs,
        roles,
        owners,
        curiosity,
        likes,
        home,
        hunger,
        dependence,
        fatigue,
        routines,
    })
}

/// Parse a deposit: `made_of:M, size:XxYxZ, per_day:N, cap:N, stock:N` (Amendment A-15). Every
/// field is required.
fn parse_deposit(id: u64, value: &str, line_no: usize) -> Result<DepositSpec, ParseError> {
    let (mut made_of, mut size, mut per_day, mut cap, mut stock) = (None, None, None, None, None);
    for part in value.split(',') {
        let (k, v) = part
            .split_once(':')
            .ok_or_else(|| ParseError::at(line_no, format!("expected key:value, got {part:?}")))?;
        match k.trim() {
            "made_of" => made_of = Some(parse_num(v.trim(), line_no)?),
            "size" => {
                let dims = v
                    .trim()
                    .split('x')
                    .map(|n| parse_num(n, line_no))
                    .collect::<Result<Vec<i64>, _>>()?;
                let [x, y, z] = dims[..] else {
                    return Err(ParseError::at(line_no, "a size is XxYxZ"));
                };
                size = Some([x, y, z]);
            }
            "per_day" => per_day = Some(parse_num(v.trim(), line_no)?),
            "cap" => cap = Some(parse_num(v.trim(), line_no)?),
            "stock" => stock = Some(parse_num(v.trim(), line_no)?),
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown deposit field {other:?}"),
                ))
            }
        }
    }
    let missing = |name: &str| ParseError::at(line_no, format!("deposit {id} lacks {name}"));
    Ok(DepositSpec {
        id,
        made_of: made_of.ok_or_else(|| missing("made_of"))?,
        size: size.ok_or_else(|| missing("size"))?,
        per_day: per_day.ok_or_else(|| missing("per_day"))?,
        cap: cap.ok_or_else(|| missing("cap"))?,
        stock: stock.ok_or_else(|| missing("stock"))?,
    })
}

/// Parse a recipe: `needs:MxN MxN, makes:M, size:XxYxZ, at:E, takes:S` (Amendment A-17) — so
/// many things of each material, the product's material and size, the workplace, and the
/// seconds in the making. Every field is required.
fn parse_recipe(id: u64, value: &str, line_no: usize) -> Result<RecipeSpec, ParseError> {
    let (mut needs, mut makes, mut size, mut at, mut takes) = (None, None, None, None, None);
    let pair = |s: &str| -> Result<(i64, i64), ParseError> {
        let (a, b) = s
            .split_once('x')
            .ok_or_else(|| ParseError::at(line_no, format!("expected MxN, got {s:?}")))?;
        Ok((parse_num(a, line_no)?, parse_num(b, line_no)?))
    };
    for part in value.split(',') {
        let (k, v) = part
            .split_once(':')
            .ok_or_else(|| ParseError::at(line_no, format!("expected key:value, got {part:?}")))?;
        match k.trim() {
            "needs" => {
                let mut list = Vec::new();
                for item in v.split_whitespace() {
                    let (material, count) = pair(item)?;
                    if material < 0 || count <= 0 {
                        return Err(ParseError::at(line_no, "a recipe needs some of a material"));
                    }
                    list.push((material as u64, count));
                }
                needs = Some(list);
            }
            "makes" => makes = Some(parse_num(v.trim(), line_no)?),
            "size" => {
                let dims = v
                    .trim()
                    .split('x')
                    .map(|n| parse_num(n, line_no))
                    .collect::<Result<Vec<i64>, _>>()?;
                let [x, y, z] = dims[..] else {
                    return Err(ParseError::at(line_no, "a size is XxYxZ"));
                };
                size = Some([x, y, z]);
            }
            "at" => at = Some(parse_num(v.trim(), line_no)?),
            "takes" => takes = Some(parse_num(v.trim(), line_no)?),
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown recipe field {other:?}"),
                ))
            }
        }
    }
    let missing = |name: &str| ParseError::at(line_no, format!("recipe {id} lacks {name}"));
    Ok(RecipeSpec {
        id,
        needs: needs.ok_or_else(|| missing("needs"))?,
        makes: makes.ok_or_else(|| missing("makes"))?,
        size: size.ok_or_else(|| missing("size"))?,
        at: at.ok_or_else(|| missing("at"))?,
        takes_seconds: takes.ok_or_else(|| missing("takes"))?,
    })
}

/// Parse a job: `carry:M, from:E, to:P, keep:N, hours:A-B` or `make:R, to:P, keep:N, hours:A-B`
/// (Amendment A-18). The mechanism is the engine's closed set; every field it needs is required.
fn parse_job(id: u64, value: &str, line_no: usize) -> Result<JobSpec, ParseError> {
    let (mut carry, mut make, mut from, mut to, mut keep, mut hours) =
        (None, None, None, None, None, None);
    for part in value.split(',') {
        let (k, v) = part
            .split_once(':')
            .ok_or_else(|| ParseError::at(line_no, format!("expected key:value, got {part:?}")))?;
        let v = v.trim();
        match k.trim() {
            "carry" => carry = Some(parse_num(v, line_no)?),
            "make" => make = Some(parse_num(v, line_no)?),
            "from" => from = Some(parse_num(v, line_no)?),
            "to" => to = Some(parse_num(v, line_no)?),
            "keep" => keep = Some(parse_num(v, line_no)?),
            "hours" => hours = Some(parse_hours(v, line_no)?),
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown job field {other:?}"),
                ))
            }
        }
    }
    let missing = |name: &str| ParseError::at(line_no, format!("job {id} lacks {name}"));
    let work = match (carry, make) {
        (Some(material), None) => JobWork::Carry {
            material,
            from: from.ok_or_else(|| missing("from"))?,
        },
        (None, Some(recipe)) => JobWork::Make { recipe },
        _ => {
            return Err(ParseError::at(
                line_no,
                format!("job {id} must either carry or make"),
            ))
        }
    };
    Ok(JobSpec {
        id,
        work,
        to: to.ok_or_else(|| missing("to"))?,
        keep: keep.ok_or_else(|| missing("keep"))?,
        hours: hours.ok_or_else(|| missing("hours"))?,
    })
}

/// Parse a kind of need: `arises:bond|dependence, met_by:presence|dose, rise:N, ease:N, harm:N`
/// (Amendment A-11). Every field is required; the mechanisms are the engine's closed set.
fn parse_need_kind(id: u64, value: &str, line_no: usize) -> Result<NeedKindSpec, ParseError> {
    let (mut arises, mut met_by, mut rise, mut ease, mut harm) = (None, None, None, None, None);
    let mut above = None;
    for part in value.split(',') {
        let (k, v) = part
            .split_once(':')
            .ok_or_else(|| ParseError::at(line_no, format!("expected key:value, got {part:?}")))?;
        match (k.trim(), v.trim()) {
            ("arises", "bond") => arises = Some(1),
            ("arises", "dependence") => arises = Some(2),
            ("met_by", "presence") => met_by = Some(1),
            ("met_by", "dose") => met_by = Some(2),
            ("rise", n) => rise = Some(parse_num(n, line_no)?),
            ("ease", n) => ease = Some(parse_num(n, line_no)?),
            ("harm", n) => harm = Some(parse_num(n, line_no)?),
            ("above", n) => above = Some(parse_num(n, line_no)?),
            (k, v) => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown need-kind field {k}:{v}"),
                ))
            }
        }
    }
    let field = |v: Option<i64>, name: &str| {
        v.ok_or_else(|| ParseError::at(line_no, format!("need kind {id} lacks {name}")))
    };
    let arises = field(arises, "arises")?;
    // A kind that arises from dependence must say past what dependence it arises.
    let above = if arises == 2 {
        field(above, "above")?
    } else {
        above.unwrap_or(0)
    };
    Ok(NeedKindSpec {
        id,
        above,
        arises,
        met_by: field(met_by, "met_by")?,
        rise: field(rise, "rise")?,
        ease: field(ease, "ease")?,
        harm: field(harm, "harm")?,
    })
}

/// Parse exactly `N` comma-separated integers, naming the expected shape in any error.
fn parse_ints<const N: usize>(
    value: &str,
    line_no: usize,
    shape: &str,
) -> Result<[i64; N], ParseError> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != N {
        return Err(ParseError::at(line_no, format!("expected `{shape}`")));
    }
    let mut out = [0i64; N];
    for (slot, part) in out.iter_mut().zip(parts) {
        *slot = parse_num(part, line_no)?;
    }
    Ok(out)
}

/// Parse a compass bearing in degrees — a whole number or up to two decimal places, possibly
/// negative (`90`, `22.5`, `-45`) — into hundredths of a degree.
fn parse_degrees(value: &str, line_no: usize) -> Result<i64, ParseError> {
    let v = value.trim();
    let bad = || {
        ParseError::at(
            line_no,
            format!("expected degrees like 90 or 22.5, got {v:?}"),
        )
    };
    let (negative, digits) = match v.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, v),
    };
    // A decimal point must have digits after it: "1." is a typo, not a bearing.
    let (whole, frac) = match digits.split_once('.') {
        Some((_, "")) => return Err(bad()),
        Some((w, f)) => (w, f),
        None => (digits, ""),
    };
    if whole.is_empty() || frac.len() > 2 || !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(bad());
    }
    let whole: i64 = whole.parse().map_err(|_| bad())?;
    let frac: i64 = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<2}").parse().map_err(|_| bad())?
    };
    let centi = whole
        .checked_mul(100)
        .and_then(|w| w.checked_add(frac))
        .ok_or_else(bad)?;
    Ok(if negative { -centi } else { centi })
}

/// Parse a material line's value: a comma-separated list of `property:value` pairs, e.g.
/// `density:700, hardness:3000, flammability:7000`. A material declares only the properties it
/// exposes (Vol. III Ch. 1 §1.9); an unknown property name is an error, never ignored.
fn parse_material_properties(
    value: &str,
    line_no: usize,
) -> Result<Vec<(MaterialProperty, i64)>, ParseError> {
    let mut out = Vec::new();
    for pair in value.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let colon = pair
            .find(':')
            .ok_or_else(|| ParseError::at(line_no, "expected `property:value`"))?;
        let name = pair[..colon].trim();
        let raw = pair[colon + 1..].trim();
        let property = match name {
            "density" => MaterialProperty::Density,
            "hardness" => MaterialProperty::Hardness,
            "thermal_capacity" => MaterialProperty::ThermalCapacity,
            "flammability" => MaterialProperty::Flammability,
            "conductivity" => MaterialProperty::Conductivity,
            "toxicity" => MaterialProperty::Toxicity,
            "edible" => MaterialProperty::Edible,
            "potency" => MaterialProperty::Potency,
            "habit" => MaterialProperty::Habit,
            "nutrition" => MaterialProperty::Nutrition,
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown material property {other:?}"),
                ))
            }
        };
        out.push((property, parse_num(raw, line_no)?));
    }
    Ok(out)
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn split_kv(line: &str, line_no: usize) -> Result<(&str, &str), ParseError> {
    let idx = line
        .find('=')
        .ok_or_else(|| ParseError::at(line_no, "expected `key = value`"))?;
    let key = line[..idx].trim();
    let value = line[idx + 1..].trim();
    if key.is_empty() {
        return Err(ParseError::at(line_no, "empty key"));
    }
    Ok((key, value))
}

/// Parse `temperature` or `temperature, elevation` for a region line.
fn parse_region_values(value: &str, line_no: usize) -> Result<(i64, Option<i64>), ParseError> {
    let mut parts = value.split(',');
    let temp = parse_num(
        parts
            .next()
            .ok_or_else(|| ParseError::at(line_no, "expected a temperature"))?,
        line_no,
    )?;
    let elevation = match parts.next() {
        Some(e) => Some(parse_num(e, line_no)?),
        None => None,
    };
    if parts.next().is_some() {
        return Err(ParseError::at(
            line_no,
            "expected `temperature` or `temperature, elevation`",
        ));
    }
    Ok((temp, elevation))
}

/// Parse `x, y` or `x, y, z` for a position line (z optional).
fn parse_position_values(
    value: &str,
    line_no: usize,
) -> Result<(i64, i64, Option<i64>), ParseError> {
    let mut parts = value.split(',');
    let x = parse_num(
        parts
            .next()
            .ok_or_else(|| ParseError::at(line_no, "expected `x, y[, z]`"))?,
        line_no,
    )?;
    let y = parse_num(
        parts
            .next()
            .ok_or_else(|| ParseError::at(line_no, "expected `x, y[, z]`"))?,
        line_no,
    )?;
    let z = match parts.next() {
        Some(zs) => Some(parse_num(zs, line_no)?),
        None => None,
    };
    if parts.next().is_some() {
        return Err(ParseError::at(line_no, "expected `x, y` or `x, y, z`"));
    }
    Ok((x, y, z))
}

/// Parse `host, dest, x, y` or `host, dest, x, y, z` for a portal line (z optional).
fn parse_portal_values(
    value: &str,
    line_no: usize,
) -> Result<(u64, u64, i64, i64, Option<i64>), ParseError> {
    let mut parts = value.split(',');
    let mut next = |what: &str| -> Result<&str, ParseError> {
        parts.next().ok_or_else(|| {
            ParseError::at(
                line_no,
                format!("expected `host, dest, x, y[, z]` ({what})"),
            )
        })
    };
    let host: u64 = parse_num(next("host")?, line_no)?;
    let dest: u64 = parse_num(next("dest")?, line_no)?;
    let x: i64 = parse_num(next("x")?, line_no)?;
    let y: i64 = parse_num(next("y")?, line_no)?;
    let z = match parts.next() {
        Some(zs) => Some(parse_num(zs, line_no)?),
        None => None,
    };
    if parts.next().is_some() {
        return Err(ParseError::at(
            line_no,
            "expected `host, dest, x, y` or `host, dest, x, y, z`",
        ));
    }
    Ok((host, dest, x, y, z))
}

/// Parse a `"a, b"` pair of numbers (used for `organism_id = region_id, body_heat`).
fn split_pair(value: &str, line_no: usize) -> Result<(u64, i64), ParseError> {
    let mut parts = value.split(',');
    let a = parts
        .next()
        .ok_or_else(|| ParseError::at(line_no, "expected `region_id, body_heat`"))?;
    let b = parts
        .next()
        .ok_or_else(|| ParseError::at(line_no, "expected `region_id, body_heat`"))?;
    if parts.next().is_some() {
        return Err(ParseError::at(
            line_no,
            "expected exactly `region_id, body_heat`",
        ));
    }
    Ok((parse_num(a, line_no)?, parse_num(b, line_no)?))
}

fn parse_num<T: FromStr>(value: &str, line_no: usize) -> Result<T, ParseError> {
    value
        .trim()
        .parse()
        .map_err(|_| ParseError::at(line_no, format!("expected a number, got {value:?}")))
}

/// Parse hours of the day `FROM-TO`, each a whole hour 0–24; the window wraps past midnight when
/// `FROM > TO`.
fn parse_hours(value: &str, line_no: usize) -> Result<(i64, i64), ParseError> {
    let (a, b) = value
        .trim()
        .split_once('-')
        .ok_or_else(|| ParseError::at(line_no, "hours are FROM-TO"))?;
    let (a, b): (i64, i64) = (parse_num(a, line_no)?, parse_num(b, line_no)?);
    if !(0..=24).contains(&a) || !(0..=24).contains(&b) {
        return Err(ParseError::at(line_no, "hours lie within a day"));
    }
    Ok((a, b))
}

fn require<T>(opt: Option<T>, what: &str) -> Result<T, ParseError> {
    opt.ok_or_else(|| ParseError::at(0, format!("missing required field {what}")))
}

#[cfg(test)]
mod tests {
    use super::parse_world;
    use crate::model::{Flag, MaterialProperty};

    const HEADER: &str = "\
[manifest]
id = world.test
version = 0.1.0
engine = >=0.0, <1.0
domains = physical
[clock]
tick_seconds = 3600
day_seconds = 86400
[rules.physical]
environment_step_seconds = 3600
diurnal_amplitude_centi_c = 400
temperature_variability_centi_c = 300
weather_persistence_seconds = 21600
illumination_peak = 10000
humidity_baseline = 5500
humidity_variability = 800
pressure_sea_level = 10130
pressure_elevation_factor = 1
pressure_variability = 60
wind_gradient_divisor = 10
fall_danger_per_meter = 1500
thermal_mass_reference = 1000
gravity_cm_s2 = 981
step_height_cm = 40
max_slope_percent = 100
nav_cell_cm = 50
reach_cm = 75
indoor_coupling_seconds = 14400
sight_step_seconds = 1
sight_min_illumination = 50
carry_limit_kg = 25
[regions]
1 = 1500
";

    #[test]
    fn materials_and_composition_parse() {
        let text = format!(
            "{HEADER}\
[materials]
700 = density:700, hardness:3000, flammability:7000
[made_of]
1 = 700
"
        );
        let pkg = parse_world(&text).expect("parses");
        assert_eq!(pkg.materials.len(), 1);
        assert_eq!(pkg.materials[0].id, 700);
        assert_eq!(
            pkg.materials[0].properties,
            vec![
                (MaterialProperty::Density, 700),
                (MaterialProperty::Hardness, 3000),
                (MaterialProperty::Flammability, 7000),
            ]
        );
        assert_eq!(pkg.made_of.len(), 1);
        assert_eq!(pkg.made_of[0].object_id, 1);
        assert_eq!(pkg.made_of[0].material_id, 700);
    }

    #[test]
    fn a_recipe_parses_what_it_needs_makes_where_and_how_long() {
        let text = format!(
            "{HEADER}\
[recipes]
950 = needs:704x2 706x1, makes:707, size:10x10x5, at:122, takes:3600   # a pie
"
        );
        let pkg = parse_world(&text).expect("parses");
        let r = &pkg.recipes[0];
        assert_eq!(r.id, 950);
        assert_eq!(r.needs, vec![(704, 2), (706, 1)]);
        assert_eq!(
            (r.makes, r.size, r.at, r.takes_seconds),
            (707, [10, 10, 5], 122, 3600)
        );
    }

    #[test]
    fn a_recipe_without_a_workplace_is_refused() {
        let text = format!(
            "{HEADER}\
[recipes]
950 = needs:704x2, makes:707, size:10x10x5, takes:3600
"
        );
        let err = parse_world(&text).expect_err("no workplace");
        assert!(err.reason.contains("lacks at"), "got: {}", err.reason);
    }

    #[test]
    fn wants_parse_likes_homes_owners_and_curiosity() {
        let text = format!(
            "{HEADER}\
[owners]
121 = 2002
[curiosity]
2011 = 100
[likes]
2005 = 701:150, 702:20
[home]
2005 = 104
"
        );
        let pkg = parse_world(&text).expect("parses");
        assert_eq!(pkg.owners, vec![(121, 2002)]);
        assert_eq!(pkg.curiosity, vec![(2011, 100)]);
        assert_eq!(pkg.likes, vec![(2005, 701, 150), (2005, 702, 20)]);
        assert_eq!(pkg.home, vec![(2005, 104)]);
    }

    #[test]
    fn region_memberships_parse_one_link_per_region() {
        let text = format!(
            "{HEADER}\
[in_region]
3 = 900, 901     # the farmhouse lies in a climate zone and a watershed
4 = 902
"
        );
        let pkg = parse_world(&text).expect("parses");
        let links: Vec<(u64, u64)> = pkg
            .in_region
            .iter()
            .map(|m| (m.location_id, m.region_id))
            .collect();
        assert_eq!(links, vec![(3, 900), (3, 901), (4, 902)]);
    }

    #[test]
    fn a_location_cannot_be_its_own_region() {
        let text = format!(
            "{HEADER}\
[in_region]
5 = 900, 5
"
        );
        let err = parse_world(&text).expect_err("must reject self-membership");
        assert!(
            err.reason.contains("region of itself"),
            "got: {}",
            err.reason
        );
    }

    #[test]
    fn an_unknown_material_property_is_rejected() {
        // No silent defaults: a property the engine does not model is an error, not ignored
        // (Vol. IV Ch. 2, missing/unknown is failure).
        let text = format!(
            "{HEADER}\
[materials]
700 = density:700, sparkliness:9000
"
        );
        let err = parse_world(&text).expect_err("must reject unknown property");
        assert!(
            err.reason.contains("sparkliness"),
            "error should name the offending property, got: {}",
            err.reason
        );
    }

    #[test]
    fn the_clock_takes_seconds_or_milliseconds_but_not_both() {
        let tenth = HEADER.replace("tick_seconds = 3600", "tick_ms = 100");
        assert_eq!(parse_world(&tenth).expect("parses").clock.tick_ms, 100);
        assert_eq!(parse_world(HEADER).unwrap().clock.tick_ms, 3_600_000);
        let both = HEADER.replace("tick_seconds = 3600", "tick_seconds = 3600\ntick_ms = 100");
        assert!(parse_world(&both).is_err(), "two tick lengths is ambiguous");
        let zero = HEADER.replace("tick_seconds = 3600", "tick_seconds = 0");
        assert!(parse_world(&zero).is_err(), "time must pass");
        let none = HEADER.replace("tick_seconds = 3600\n", "");
        let err = parse_world(&none).expect_err("no default tick length");
        assert!(
            err.reason.contains("clock.tick_seconds"),
            "got {}",
            err.reason
        );
    }

    #[test]
    fn bodies_facing_and_motion_parse() {
        let text = format!(
            "{HEADER}\
[bodies]
10 = 25, 15, 175      # a person: 50 cm wide, 30 cm deep, 1.75 m tall
[facing]
10 = 90
11 = 22.5
12 = -45
[motion]
10 = 1000, 0, 0, 30
"
        );
        let pkg = parse_world(&text).expect("parses");
        assert_eq!(
            pkg.bodies,
            vec![crate::model::BodySpec {
                entity_id: 10,
                half_width: 25,
                half_depth: 15,
                height: 175
            }]
        );
        let headings: Vec<i64> = pkg.facing.iter().map(|f| f.heading).collect();
        assert_eq!(headings, vec![9_000, 2_250, -4_500]);
        assert_eq!(pkg.motion[0].target, [1000, 0, 0]);
        assert_eq!(pkg.motion[0].seconds, 30);
        for bad in ["10 = 1.234", "10 = east", "10 = 1."] {
            let t = format!("{HEADER}[facing]\n{bad}\n");
            assert!(parse_world(&t).is_err(), "{bad} must be rejected");
        }
        let neg = format!("{HEADER}[bodies]\n10 = -1, 1, 1\n");
        assert!(parse_world(&neg).is_err(), "negative size");
    }

    #[test]
    fn constraints_terrain_and_travel_parse() {
        let text = format!(
            "{HEADER}\
[flags]
5 = enclosed
30 = solid, opaque
1000 = closed, opaque
10 = mobile
[portal_pairs]
1000 = 1001
[terrain]
1 = 500, 3, 0 10 20  5 15 25
[travel]
10 = 2, 140
"
        );
        let pkg = parse_world(&text).expect("parses");
        assert_eq!(pkg.flags.len(), 4);
        assert_eq!(pkg.flags[1].flags, vec![Flag::Solid, Flag::Opaque]);
        assert_eq!(pkg.portal_pairs, vec![(1000, 1001)]);
        assert_eq!(pkg.terrain[0].heights, vec![0, 10, 20, 5, 15, 25]);
        assert_eq!(pkg.terrain[0].columns, 3);
        assert_eq!(pkg.travel[0].target, 2);
        let ragged = format!("{HEADER}[terrain]\n1 = 500, 3, 0 10 20 5\n");
        assert!(parse_world(&ragged).is_err(), "rows must be whole");
        let unknown = format!("{HEADER}[flags]\n1 = sticky\n");
        assert!(parse_world(&unknown).is_err());
    }
}
