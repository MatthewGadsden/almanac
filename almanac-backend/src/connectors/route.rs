//! Turns a GPS route into an outline: its shape, simplified and scaled into a 100×100 box. Only
//! the shape is kept, never coordinates, so an outline doesn't reveal where an activity was.

use crate::domain::ROUTE_OUTLINE_SIZE;

const OUTLINE_SIZE: f64 = ROUTE_OUTLINE_SIZE as f64;
/// Fewer points than this aren't a route.
const MIN_POINTS: usize = 2;
/// Points kept at most; plenty for an icon, and keeps `data` small.
const MAX_POINTS: usize = 80;
/// How far (in outline units) simplification may move the line.
const TOLERANCE: f64 = 0.75;

/// The outline of a route given as an encoded polyline (Google's format, 5 decimal places, as
/// Strava's `map.summary_polyline`). `None` for an empty or malformed polyline, or a route that
/// never moved (e.g. a treadmill run).
pub fn outline_from_polyline(encoded: &str) -> Option<Vec<[u8; 2]>> {
	outline(&decode_polyline(encoded)?)
}

/// Decodes an encoded polyline into (latitude, longitude) pairs.
fn decode_polyline(encoded: &str) -> Option<Vec<(f64, f64)>> {
	let mut bytes = encoded.bytes();
	let mut next_value = || -> Option<Option<i64>> {
		let mut result: i64 = 0;
		let mut shift = 0;
		loop {
			let Some(byte) = bytes.next() else {
				// The end of the string is only valid between values
				return if shift == 0 { Some(None) } else { None };
			};
			let chunk = i64::from(byte.checked_sub(63).filter(|c| *c < 64)?);
			result |= (chunk & 0x1f) << shift;
			shift += 5;
			if chunk < 0x20 {
				break;
			}
			if shift > 35 {
				return None;
			}
		}
		let value = if result & 1 == 1 {
			!(result >> 1)
		} else {
			result >> 1
		};
		Some(Some(value))
	};

	let (mut lat, mut lng) = (0i64, 0i64);
	let mut points = Vec::new();
	while let Some(d_lat) = next_value()? {
		let d_lng = next_value()??;
		lat += d_lat;
		lng += d_lng;
		points.push((lat as f64 / 1e5, lng as f64 / 1e5));
	}
	Some(points)
}

/// Projects, simplifies and scales a route into the outline box, centred, keeping its aspect.
fn outline(points: &[(f64, f64)]) -> Option<Vec<[u8; 2]>> {
	if points.len() < MIN_POINTS {
		return None;
	}

	// Equirectangular projection: fine at the scale of one activity. y grows downwards, as in SVG.
	let mid_lat = points.iter().map(|(lat, _)| lat).sum::<f64>() / points.len() as f64;
	let x_scale = mid_lat.to_radians().cos();
	let projected: Vec<(f64, f64)> = points
		.iter()
		.map(|(lat, lng)| (lng * x_scale, -lat))
		.collect();

	let (min_x, max_x) = bounds(projected.iter().map(|p| p.0));
	let (min_y, max_y) = bounds(projected.iter().map(|p| p.1));
	let extent = (max_x - min_x).max(max_y - min_y);
	if extent <= 0.0 || !extent.is_finite() {
		return None;
	}

	let scale = OUTLINE_SIZE / extent;
	let offset_x = (OUTLINE_SIZE - (max_x - min_x) * scale) / 2.0;
	let offset_y = (OUTLINE_SIZE - (max_y - min_y) * scale) / 2.0;
	let scaled: Vec<(f64, f64)> = projected
		.iter()
		.map(|(x, y)| {
			(
				(x - min_x) * scale + offset_x,
				(y - min_y) * scale + offset_y,
			)
		})
		.collect();

	// Simplify, loosening the tolerance until the outline is small enough
	let mut tolerance = TOLERANCE;
	let mut simplified = simplify(&scaled, tolerance);
	while simplified.len() > MAX_POINTS {
		tolerance *= 1.5;
		simplified = simplify(&scaled, tolerance);
	}

	let mut outline: Vec<[u8; 2]> = Vec::with_capacity(simplified.len());
	for (x, y) in simplified {
		let point = [round_to_box(x), round_to_box(y)];
		if outline.last() != Some(&point) {
			outline.push(point);
		}
	}
	(outline.len() >= MIN_POINTS).then_some(outline)
}

fn bounds(values: impl Iterator<Item = f64>) -> (f64, f64) {
	values.fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), v| {
		(min.min(v), max.max(v))
	})
}

fn round_to_box(value: f64) -> u8 {
	value.round().clamp(0.0, OUTLINE_SIZE) as u8
}

/// Ramer–Douglas–Peucker: keeps the points that stray more than `tolerance` from the line
/// between the points kept either side of them.
fn simplify(points: &[(f64, f64)], tolerance: f64) -> Vec<(f64, f64)> {
	if points.len() < 3 {
		return points.to_vec();
	}

	let mut keep = vec![false; points.len()];
	keep[0] = true;
	keep[points.len() - 1] = true;
	let mut stack = vec![(0, points.len() - 1)];

	while let Some((start, end)) = stack.pop() {
		let farthest = (start + 1..end)
			.map(|i| {
				(
					i,
					distance_to_segment(points[i], points[start], points[end]),
				)
			})
			.max_by(|a, b| a.1.total_cmp(&b.1));
		if let Some((index, distance)) = farthest
			&& distance > tolerance
		{
			keep[index] = true;
			stack.push((start, index));
			stack.push((index, end));
		}
	}

	points
		.iter()
		.zip(keep)
		.filter_map(|(point, kept)| kept.then_some(*point))
		.collect()
}

fn distance_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
	let (dx, dy) = (b.0 - a.0, b.1 - a.1);
	let length_squared = dx * dx + dy * dy;
	let t = if length_squared == 0.0 {
		0.0
	} else {
		(((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_squared).clamp(0.0, 1.0)
	};
	let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
	((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The example from Google's polyline documentation.
	const GOOGLE_EXAMPLE: &str = "_p~iF~ps|U_ulLnnqC_mqNvxq`@";

	#[test]
	fn decodes_polylines() {
		let points = decode_polyline(GOOGLE_EXAMPLE).unwrap();
		let expected = [(38.5, -120.2), (40.7, -120.95), (43.252, -126.453)];
		assert_eq!(points.len(), expected.len());
		for ((lat, lng), (expected_lat, expected_lng)) in points.iter().zip(expected) {
			assert!((lat - expected_lat).abs() < 1e-9 && (lng - expected_lng).abs() < 1e-9);
		}
	}

	#[test]
	fn rejects_malformed_polylines() {
		assert_eq!(decode_polyline(""), Some(vec![]));
		assert_eq!(decode_polyline("abc"), None, "ends mid-value");
		assert_eq!(
			decode_polyline("_p~iF"),
			None,
			"a latitude with no longitude"
		);
		assert_eq!(
			decode_polyline("_p~iF ~ps|U"),
			None,
			"a character outside the alphabet"
		);
		assert_eq!(outline_from_polyline(""), None);
	}

	#[test]
	fn outlines_fill_the_box_and_keep_their_aspect() {
		// East for twice as far as it then goes north
		let route = [(0.0, 0.0), (0.0, 0.02), (0.01, 0.02)];

		// The width spans the box, the half-as-tall route is centred, and north is up
		assert_eq!(outline(&route), Some(vec![[0, 75], [100, 75], [100, 25]]));
	}

	#[test]
	fn a_route_that_never_moved_has_no_outline() {
		assert_eq!(outline(&[(51.5, -0.1), (51.5, -0.1), (51.5, -0.1)]), None);
		assert_eq!(outline(&[(51.5, -0.1)]), None);
	}

	#[test]
	fn long_routes_are_simplified() {
		// A circle of 2,000 points
		let circle: Vec<(f64, f64)> = (0..2000)
			.map(|i| {
				let angle = i as f64 / 2000.0 * std::f64::consts::TAU;
				(51.5 + 0.01 * angle.sin(), -0.1 + 0.016 * angle.cos())
			})
			.collect();
		let outline = outline(&circle).unwrap();
		assert!(
			(8..=MAX_POINTS).contains(&outline.len()),
			"{} points",
			outline.len()
		);
	}
}
