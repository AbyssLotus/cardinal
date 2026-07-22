//! A minimal, dependency-free parser for the world-file format (Vol. IV Ch. 1 §1.2).
//!
//! The package layout is "conceptual, not prescriptive" (Vol. IV Ch. 1 §1.2), so this is one
//! concrete surface: a small sectioned `key = value` text format, parsed with only the
//! standard library so the engine keeps its zero-dependency, offline build. Packages remain
//! pure data — this reads declarations, it never executes them (Vol. IV Ch. 1, invariant 6).

use crate::model::{
    AdjacencySpec, ContainmentSpec, ExposureSpec, LivingRules, MadeOfSpec, Manifest,
    MaterialProperty, MaterialSpec, OrganismSpec, PhysicalRules, PortalDangerSpec, PortalSpec,
    PositionSpec, RegionSpec, WorldPackage,
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
/// Recognised sections: `[manifest]`, `[rules.physical]`, `[rules.living]`, `[regions]`
/// (`region_id = temperature[, elevation]`), `[organisms]`
/// (`organism_id = region_id, body_heat`), `[containment]` (`child_id = parent_id`),
/// `[adjacency]`, `[exposure]`, `[positions]`, `[portals]`, `[portal_danger]`, `[materials]`
/// (`material_id = property:value, …`), and `[made_of]` (`object_id = material_id[, …]`).
/// Blank lines and `#` comments are ignored. A missing required field is an error — the
/// loader never fabricates defaults (Vol. IV Ch. 2).
pub fn parse_world(text: &str) -> Result<WorldPackage, ParseError> {
    let mut section = String::new();
    let mut id: Option<String> = None;
    let mut version: Option<Version> = None;
    let mut engine: Option<EngineReq> = None;
    let mut domains: Option<Vec<String>> = None;
    let mut ticks_per_day: Option<u64> = None;
    let mut amplitude: Option<i64> = None;
    let mut swing: Option<i64> = None;
    let mut illumination_peak: Option<i64> = None;
    let mut humidity_baseline: Option<i64> = None;
    let mut humidity_swing: Option<i64> = None;
    let mut humidity_drying_divisor: Option<i64> = None;
    let mut pressure_sea_level: Option<i64> = None;
    let mut pressure_elevation_factor: Option<i64> = None;
    let mut pressure_weather_swing: Option<i64> = None;
    let mut pressure_settle_divisor: Option<i64> = None;
    let mut wind_gradient_divisor: Option<i64> = None;
    let mut fall_danger_per_meter: Option<i64> = None;
    let mut set_point: Option<i64> = None;
    let mut warm_response: Option<i64> = None;
    let mut cold_response: Option<i64> = None;
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
            "rules.physical" => match key {
                "ticks_per_day" => ticks_per_day = Some(parse_num(value, line_no)?),
                "diurnal_amplitude_centi_c" => amplitude = Some(parse_num(value, line_no)?),
                "weather_max_swing_centi_c" => swing = Some(parse_num(value, line_no)?),
                "illumination_peak" => illumination_peak = Some(parse_num(value, line_no)?),
                "humidity_baseline" => humidity_baseline = Some(parse_num(value, line_no)?),
                "humidity_swing" => humidity_swing = Some(parse_num(value, line_no)?),
                "humidity_drying_divisor" => {
                    humidity_drying_divisor = Some(parse_num(value, line_no)?)
                }
                "pressure_sea_level" => pressure_sea_level = Some(parse_num(value, line_no)?),
                "pressure_elevation_factor" => {
                    pressure_elevation_factor = Some(parse_num(value, line_no)?)
                }
                "pressure_weather_swing" => {
                    pressure_weather_swing = Some(parse_num(value, line_no)?)
                }
                "pressure_settle_divisor" => {
                    pressure_settle_divisor = Some(parse_num(value, line_no)?)
                }
                "wind_gradient_divisor" => wind_gradient_divisor = Some(parse_num(value, line_no)?),
                "fall_danger_per_meter" => fall_danger_per_meter = Some(parse_num(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown physical rule {other:?}"),
                    ))
                }
            },
            "rules.living" => match key {
                "set_point_centi_c" => set_point = Some(parse_num(value, line_no)?),
                "warm_response" => warm_response = Some(parse_num(value, line_no)?),
                "cold_response" => cold_response = Some(parse_num(value, line_no)?),
                other => {
                    return Err(ParseError::at(
                        line_no,
                        format!("unknown living rule {other:?}"),
                    ))
                }
            },
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
    let physical_rules = PhysicalRules {
        ticks_per_day: require(ticks_per_day, "rules.physical.ticks_per_day")?,
        diurnal_amplitude_centi_c: require(amplitude, "rules.physical.diurnal_amplitude_centi_c")?,
        weather_max_swing_centi_c: require(swing, "rules.physical.weather_max_swing_centi_c")?,
        illumination_peak: require(illumination_peak, "rules.physical.illumination_peak")?,
        humidity_baseline: require(humidity_baseline, "rules.physical.humidity_baseline")?,
        humidity_swing: require(humidity_swing, "rules.physical.humidity_swing")?,
        humidity_drying_divisor: require(
            humidity_drying_divisor,
            "rules.physical.humidity_drying_divisor",
        )?,
        pressure_sea_level: require(pressure_sea_level, "rules.physical.pressure_sea_level")?,
        pressure_elevation_factor: require(
            pressure_elevation_factor,
            "rules.physical.pressure_elevation_factor",
        )?,
        pressure_weather_swing: require(
            pressure_weather_swing,
            "rules.physical.pressure_weather_swing",
        )?,
        pressure_settle_divisor: require(
            pressure_settle_divisor,
            "rules.physical.pressure_settle_divisor",
        )?,
        wind_gradient_divisor: require(
            wind_gradient_divisor,
            "rules.physical.wind_gradient_divisor",
        )?,
        fall_danger_per_meter: require(
            fall_danger_per_meter,
            "rules.physical.fall_danger_per_meter",
        )?,
    };
    let living_rules = match (set_point, warm_response, cold_response) {
        (None, None, None) => None,
        _ => Some(LivingRules {
            set_point_centi_c: require(set_point, "rules.living.set_point_centi_c")?,
            warm_response: require(warm_response, "rules.living.warm_response")?,
            cold_response: require(cold_response, "rules.living.cold_response")?,
        }),
    };

    Ok(WorldPackage {
        manifest,
        physical_rules,
        living_rules,
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
    })
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

fn require<T>(opt: Option<T>, what: &str) -> Result<T, ParseError> {
    opt.ok_or_else(|| ParseError::at(0, format!("missing required field {what}")))
}

#[cfg(test)]
mod tests {
    use super::parse_world;
    use crate::model::MaterialProperty;

    const HEADER: &str = "\
[manifest]
id = world.test
version = 0.1.0
engine = >=0.0, <1.0
domains = physical
[rules.physical]
ticks_per_day = 24
diurnal_amplitude_centi_c = 400
weather_max_swing_centi_c = 40
illumination_peak = 10000
humidity_baseline = 5500
humidity_swing = 80
humidity_drying_divisor = 8
pressure_sea_level = 10130
pressure_elevation_factor = 1
pressure_weather_swing = 20
pressure_settle_divisor = 8
wind_gradient_divisor = 10
fall_danger_per_meter = 1500
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
}
