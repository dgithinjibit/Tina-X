//! GNSS position ingest → swarm navigation input (G4D-RR bridge #2; see
//! `docs/research/g4drr-gnss-eo-bridge.md`).
//!
//! # What this is
//! The swarm coordinates on an abstract grid, but a real deployment needs to know WHERE each agent
//! actually is. This module turns raw GNSS output into positions the swarm can use:
//!   * a dependency-free parser for the two **NMEA 0183** sentences that carry a fix — `GGA`
//!     (position + fix quality) and `RMC` (position + validity + speed) — the lowest common
//!     denominator emitted by essentially every receiver (u-blox, etc.);
//!   * a [`GnssFix`] (lat/lon in decimal degrees + quality) and a [`GnssTrack`] of fixes;
//!   * a [`GeoBounds`] → grid mapping so a fix becomes an integer swarm cell `(x, y)`.
//!
//! # Why NMEA first (and RTKLIB later)
//! The research names **RTKLIB** as the full ingest engine (RTK/PPP, Galileo, RINEX/RTCM). RTKLIB is
//! C, so it belongs behind a subprocess/FFI bridge — an opt-in step. But RTKLIB's *real-time output*
//! is NMEA, and every consumer GNSS module speaks NMEA directly, so a pure-Rust NMEA parser is the
//! honest, dependency-free first slice that already ingests real receiver data (and RTKLIB output)
//! today. It's offline-first exactly like the GEOGLOWS bridge: parsing is pure and unit-tested; a
//! live serial/RTKLIB source drops in on top without changing this core.
//!
//! Galileo relevance: NMEA `GNGGA`/`GNRMC` (talker id `GN`) is the multi-constellation form emitted
//! when Galileo (and others) are used in the fix — so ingesting `GN` sentences IS ingesting Galileo.

/// One GNSS position fix in decimal degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnssFix {
    /// Latitude in decimal degrees, positive north.
    pub lat: f64,
    /// Longitude in decimal degrees, positive east.
    pub lon: f64,
    /// Fix quality: `false` = no/invalid fix (do not navigate on it), `true` = valid.
    pub valid: bool,
}

/// A geographic bounding box for the operating area, used to map lat/lon fixes onto the swarm grid.
/// `(min_lat, min_lon)` is the south-west corner, `(max_lat, max_lon)` the north-east.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoBounds {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

impl GeoBounds {
    /// Map a fix to a grid cell `(x, y)` in a `width`×`height` grid. Longitude → column (west→east),
    /// latitude → row (south→north, so row 0 is the south edge). A fix outside the box is CLAMPED to
    /// the edge cell (never panics, never returns out-of-range). Degenerate bounds (zero span) map
    /// everything to 0 on that axis. Returns `None` only for an invalid fix.
    pub fn cell(&self, fix: &GnssFix, width: usize, height: usize) -> Option<(usize, usize)> {
        if !fix.valid || width == 0 || height == 0 {
            return None;
        }
        let fx = frac(fix.lon, self.min_lon, self.max_lon);
        let fy = frac(fix.lat, self.min_lat, self.max_lat);
        let x = ((fx * width as f64) as isize).clamp(0, width as isize - 1) as usize;
        let y = ((fy * height as f64) as isize).clamp(0, height as isize - 1) as usize;
        Some((x, y))
    }
}

/// Position of `v` within `[lo, hi]` as a fraction in `[0,1]` (clamped). Zero span → 0.
fn frac(v: f64, lo: f64, hi: f64) -> f64 {
    let span = hi - lo;
    if span <= 0.0 {
        return 0.0;
    }
    ((v - lo) / span).clamp(0.0, 1.0)
}

/// A sequence of GNSS fixes (e.g. a flight track parsed from a stream of NMEA sentences).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GnssTrack {
    pub fixes: Vec<GnssFix>,
}

impl GnssTrack {
    pub fn new() -> Self {
        Self::default()
    }

    /// The most recent VALID fix, if any — the position the swarm would navigate on right now.
    pub fn latest_valid(&self) -> Option<GnssFix> {
        self.fixes.iter().rev().copied().find(|f| f.valid)
    }

    /// Parse a whole block of NMEA text (one sentence per line, `\r\n` or `\n`), collecting every
    /// GGA/RMC fix (valid or not) into a track. Non-position sentences and malformed lines are
    /// skipped rather than erroring — real receiver streams interleave many sentence types.
    pub fn from_nmea(text: &str) -> Self {
        let fixes = text.lines().filter_map(parse_nmea_line).collect();
        Self { fixes }
    }
}

/// Parse one NMEA sentence into a [`GnssFix`] if it is a GGA or RMC line. Returns `None` for any
/// other sentence type or a malformed line. Talker id is ignored (`GP`/`GN`/`GA`/… all accepted),
/// so multi-constellation `GN` sentences (which include Galileo) parse the same as GPS-only ones.
pub fn parse_nmea_line(line: &str) -> Option<GnssFix> {
    let line = line.trim();
    // Drop the leading '$' and any '*checksum' suffix; we validate structure, not the checksum
    // (an offline fixture / RTKLIB output is trusted; a live serial reader can check it upstream).
    let body = line.strip_prefix('$')?;
    let body = body.split('*').next()?;
    let f: Vec<&str> = body.split(',').collect();
    let kind = f.first()?;
    // Match on the 3-letter sentence type, ignoring the 2-letter talker id prefix.
    if kind.len() < 5 {
        return None;
    }
    match &kind[2..5] {
        "GGA" => parse_gga(&f),
        "RMC" => parse_rmc(&f),
        _ => None,
    }
}

/// GGA: `$--GGA,time,lat,N/S,lon,E/W,fixQuality,numSats,...`. Fields 2-5 carry position; field 6 is
/// fix quality (0 = invalid).
fn parse_gga(f: &[&str]) -> Option<GnssFix> {
    let lat = parse_lat(f.get(2)?, f.get(3)?)?;
    let lon = parse_lon(f.get(4)?, f.get(5)?)?;
    let quality: u8 = f.get(6)?.parse().ok()?;
    Some(GnssFix { lat, lon, valid: quality > 0 })
}

/// RMC: `$--RMC,time,status,lat,N/S,lon,E/W,speed,course,date,...`. Field 2 status is `A` (active)
/// or `V` (void); fields 3-6 carry position.
fn parse_rmc(f: &[&str]) -> Option<GnssFix> {
    let status = *f.get(2)?;
    let lat = parse_lat(f.get(3)?, f.get(4)?)?;
    let lon = parse_lon(f.get(5)?, f.get(6)?)?;
    Some(GnssFix { lat, lon, valid: status == "A" })
}

/// Parse an NMEA latitude field `ddmm.mmmm` + hemisphere (`N`/`S`) into decimal degrees.
fn parse_lat(value: &str, hemi: &str) -> Option<f64> {
    parse_dm(value, 2).map(|d| if hemi == "S" { -d } else { d })
}

/// Parse an NMEA longitude field `dddmm.mmmm` + hemisphere (`E`/`W`) into decimal degrees.
fn parse_lon(value: &str, hemi: &str) -> Option<f64> {
    parse_dm(value, 3).map(|d| if hemi == "W" { -d } else { d })
}

/// Convert an NMEA `[d]ddmm.mmmm` degrees-minutes string to decimal degrees. `deg_digits` is the
/// number of leading degree digits (2 for latitude, 3 for longitude). Empty field → None.
fn parse_dm(value: &str, deg_digits: usize) -> Option<f64> {
    if value.is_empty() || value.len() < deg_digits {
        return None;
    }
    let (deg_str, min_str) = value.split_at(deg_digits);
    let deg: f64 = deg_str.parse().ok()?;
    let min: f64 = min_str.parse().ok()?;
    Some(deg + min / 60.0)
}

/// An offline fixture NMEA stream — a short flight track of multi-constellation (`GN`, incl.
/// Galileo) fixes for demos/tests, so the GNSS bridge works without a live receiver (offline-first,
/// like the GEOGLOWS fixture). Coordinates trace a diagonal across a small box; the last fix is a
/// deliberately VOID one so consumers exercise the validity filter.
pub const FIXTURE_NMEA: &str = "\
$GNGGA,123519,4807.038,N,01131.000,E,1,10,0.8,545.4,M,46.9,M,,*4A\r
$GNRMC,123520,A,4807.200,N,01131.300,E,000.0,054.7,130998,011.3,E*6F\r
$GNGGA,123521,4807.400,N,01131.600,E,1,11,0.7,545.4,M,46.9,M,,*4C\r
$GNRMC,123522,A,4807.600,N,01131.900,E,000.0,054.7,130998,011.3,E*6D\r
$GNGGA,123523,4807.800,N,01132.200,E,1,11,0.7,545.4,M,46.9,M,,*4E\r
$GNRMC,123524,V,4808.000,N,01132.500,E,,,130998,,*3B\r";

/// A [`GeoBounds`] that comfortably contains [`FIXTURE_NMEA`] (a ~0.02°×0.03° box around the track),
/// for mapping the fixture onto a demo grid.
pub const FIXTURE_BOUNDS: GeoBounds = GeoBounds {
    min_lat: 48.11,
    min_lon: 11.51,
    max_lat: 48.14,
    max_lon: 11.55,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_parses_to_a_track_with_a_final_void_fix() {
        let track = GnssTrack::from_nmea(FIXTURE_NMEA);
        assert_eq!(track.fixes.len(), 6);
        assert!(!track.fixes.last().unwrap().valid, "last fixture fix is deliberately VOID");
        // latest_valid skips it and returns the prior (valid) fix, which maps inside the box.
        let latest = track.latest_valid().unwrap();
        assert!(FIXTURE_BOUNDS.cell(&latest, 8, 8).is_some());
    }

    #[test]
    fn parses_a_valid_gga_sentence() {
        // A real-form GGA: 48°07.038'N, 11°31.000'E, fix quality 1.
        let fix = parse_nmea_line("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47")
            .unwrap();
        assert!((fix.lat - 48.1173).abs() < 1e-3);
        assert!((fix.lon - 11.5167).abs() < 1e-3);
        assert!(fix.valid);
    }

    #[test]
    fn parses_multi_constellation_gn_talker_including_galileo() {
        // A GN talker id means multiple constellations (incl. Galileo) contributed to the fix.
        let fix = parse_nmea_line("$GNRMC,081836,A,3751.65,S,14507.36,E,000.0,360.0,130998,011.3,E*62")
            .unwrap();
        assert!(fix.valid);
        assert!(fix.lat < 0.0, "S hemisphere -> negative latitude");
        assert!(fix.lon > 0.0, "E hemisphere -> positive longitude");
    }

    #[test]
    fn marks_invalid_fixes_from_quality_and_status() {
        // GGA quality 0 = no fix; RMC status V = void. Both must be flagged invalid.
        let gga = parse_nmea_line("$GPGGA,123519,4807.038,N,01131.000,E,0,00,,,,,,,*4A").unwrap();
        assert!(!gga.valid);
        let rmc = parse_nmea_line("$GPRMC,081836,V,3751.65,S,14507.36,E,,,130998,,*3A").unwrap();
        assert!(!rmc.valid);
    }

    #[test]
    fn ignores_non_position_and_malformed_sentences() {
        assert!(parse_nmea_line("$GPGSV,3,1,11,03,03,111,00*74").is_none()); // satellites-in-view
        assert!(parse_nmea_line("garbage").is_none());
        assert!(parse_nmea_line("").is_none());
        assert!(parse_nmea_line("$GPGGA,123519,,N,,E,1,08,0.9*00").is_none()); // empty position
    }

    #[test]
    fn track_from_nmea_collects_fixes_and_finds_latest_valid() {
        let stream = "\
$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r
$GPGSV,3,1,11,03,03,111,00*74\r
$GPRMC,123520,A,4807.050,N,01131.020,E,000.0,360.0,130998,011.3,E*6A\r
$GPRMC,123521,V,4807.999,N,01131.999,E,,,130998,,*3A\r";
        let track = GnssTrack::from_nmea(stream);
        assert_eq!(track.fixes.len(), 3, "two GGA/RMC valid + one void; GSV ignored");
        // latest_valid must skip the trailing VOID fix and return the RMC before it.
        let latest = track.latest_valid().unwrap();
        assert!(latest.valid);
        assert!((latest.lat - 48.1175).abs() < 1e-3);
    }

    #[test]
    fn maps_a_fix_onto_a_grid_cell() {
        // A 10x10 grid over a 1°×1° box. A fix near the SW corner -> (0,0); near NE -> (9,9); center
        // -> middle. Latitude increases northward = increasing row.
        let b = GeoBounds { min_lat: 0.0, min_lon: 0.0, max_lat: 1.0, max_lon: 1.0 };
        let sw = GnssFix { lat: 0.01, lon: 0.01, valid: true };
        let ne = GnssFix { lat: 0.99, lon: 0.99, valid: true };
        let mid = GnssFix { lat: 0.5, lon: 0.5, valid: true };
        assert_eq!(b.cell(&sw, 10, 10), Some((0, 0)));
        assert_eq!(b.cell(&ne, 10, 10), Some((9, 9)));
        assert_eq!(b.cell(&mid, 10, 10), Some((5, 5)));
    }

    #[test]
    fn out_of_box_fixes_clamp_to_the_edge() {
        let b = GeoBounds { min_lat: 0.0, min_lon: 0.0, max_lat: 1.0, max_lon: 1.0 };
        let far = GnssFix { lat: 5.0, lon: -5.0, valid: true };
        assert_eq!(b.cell(&far, 8, 8), Some((0, 7)), "lon west of box -> col 0; lat north -> top row");
    }

    #[test]
    fn invalid_fix_maps_to_no_cell() {
        let b = GeoBounds { min_lat: 0.0, min_lon: 0.0, max_lat: 1.0, max_lon: 1.0 };
        let bad = GnssFix { lat: 0.5, lon: 0.5, valid: false };
        assert_eq!(b.cell(&bad, 8, 8), None);
    }
}
