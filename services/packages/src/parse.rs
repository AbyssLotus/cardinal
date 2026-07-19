//! A minimal, dependency-free parser for the world-file format (Vol. IV Ch. 1 §1.2).
//!
//! The package layout is "conceptual, not prescriptive" (Vol. IV Ch. 1 §1.2), so this is one
//! concrete surface: a small sectioned `key = value` text format, parsed with only the
//! standard library so the engine keeps its zero-dependency, offline build. Packages remain
//! pure data — this reads declarations, it never executes them (Vol. IV Ch. 1, invariant 6).

use crate::model::{LivingRules, Manifest, OrganismSpec, PhysicalRules, RegionSpec, WorldPackage};
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
/// (`region_id = temperature`), and `[organisms]` (`organism_id = region_id, body_heat`).
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
    let mut set_point: Option<i64> = None;
    let mut warm_response: Option<i64> = None;
    let mut cold_response: Option<i64> = None;
    let mut regions: Vec<RegionSpec> = Vec::new();
    let mut organisms: Vec<OrganismSpec> = Vec::new();

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
                let temp: i64 = parse_num(value, line_no)?;
                regions.push(RegionSpec {
                    id: region_id,
                    temperature_centi_c: temp,
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
    };
    // Living rules are optional as a block, but all-or-nothing when present: a partial
    // [rules.living] is a missing-field error, never silently defaulted (Vol. IV Ch. 2).
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
    })
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
