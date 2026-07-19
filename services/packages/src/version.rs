//! Semantic versions and engine-compatibility ranges (Vol. IV Ch. 1 §1.5, invariant 10).
//!
//! A package declares the engine versions it can run against; the loader enforces that
//! declaration rather than treating it as advisory (Vol. IV Ch. 1, invariant 10).

use std::cmp::Ordering;
use std::fmt;

/// A semantic version `major.minor.patch`. A missing patch parses as 0.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Version {
    /// Major component.
    pub major: u32,
    /// Minor component.
    pub minor: u32,
    /// Patch component.
    pub patch: u32,
}

impl Version {
    /// Construct a version from its three components.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse `"major.minor"` or `"major.minor.patch"` (a missing patch is 0).
    pub fn parse(text: &str) -> Result<Self, VersionParseError> {
        let mut parts = text.trim().split('.');
        let major = next_num(&mut parts, text)?;
        let minor = next_num(&mut parts, text)?;
        let patch = match parts.next() {
            Some(p) => p.trim().parse().map_err(|_| VersionParseError::new(text))?,
            None => 0,
        };
        if parts.next().is_some() {
            return Err(VersionParseError::new(text));
        }
        Ok(Self::new(major, minor, patch))
    }

    fn as_tuple(&self) -> (u32, u32, u32) {
        (self.major, self.minor, self.patch)
    }
}

fn next_num<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    text: &str,
) -> Result<u32, VersionParseError> {
    parts
        .next()
        .and_then(|p| p.trim().parse().ok())
        .ok_or_else(|| VersionParseError::new(text))
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_tuple().cmp(&other.as_tuple())
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A failure to parse a [`Version`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VersionParseError {
    /// The text that could not be parsed.
    pub text: String,
}

impl VersionParseError {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid version: {:?}", self.text)
    }
}

/// An engine-compatibility range: `min` inclusive, `max` exclusive (e.g. `">=0.0, <1.0"`),
/// as declared by a package manifest (Vol. IV Ch. 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EngineReq {
    /// Inclusive lower bound.
    pub min: Version,
    /// Exclusive upper bound.
    pub max: Version,
}

impl EngineReq {
    /// Construct a range from an inclusive minimum and exclusive maximum.
    pub const fn new(min: Version, max: Version) -> Self {
        Self { min, max }
    }

    /// Parse a two-part range like `">=0.3, <0.5"` (whitespace flexible).
    pub fn parse(text: &str) -> Result<Self, VersionParseError> {
        let mut min: Option<Version> = None;
        let mut max: Option<Version> = None;
        for part in text.split(',') {
            let p = part.trim();
            if let Some(rest) = p.strip_prefix(">=") {
                min = Some(Version::parse(rest)?);
            } else if let Some(rest) = p.strip_prefix('<') {
                max = Some(Version::parse(rest)?);
            } else {
                return Err(VersionParseError::new(text));
            }
        }
        match (min, max) {
            (Some(min), Some(max)) => Ok(Self::new(min, max)),
            _ => Err(VersionParseError::new(text)),
        }
    }

    /// Whether `version` satisfies this range (`min <= version < max`).
    pub fn accepts(&self, version: Version) -> bool {
        self.min <= version && version < self.max
    }
}
