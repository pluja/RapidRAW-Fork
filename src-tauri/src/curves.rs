//! Reference for the tone curves in shaders/shader.wgsl.
//!
//! Curves run after the display transform, so their domain is the 0..255 the
//! curve editor works in and their output is display-referred sRGB.
//!
//! The shader carries its own copy for the GPU. This one exists so that what a
//! curve does to a colour can be measured, and so that the luma curve keeps one
//! meaning rather than changing according to whether an RGB curve happens to be
//! off default.

#![allow(dead_code)]

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

pub const IDENTITY: [Point; 2] = [Point { x: 0.0, y: 0.0 }, Point { x: 255.0, y: 255.0 }];

/// Rec.709 weights, since this runs on data that has left the working space
/// through the display transform.
const DISPLAY_LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

pub fn display_luma(rgb: [f32; 3]) -> f32 {
    rgb[0] * DISPLAY_LUMA[0] + rgb[1] * DISPLAY_LUMA[1] + rgb[2] * DISPLAY_LUMA[2]
}

/// Whether a curve is the untouched diagonal, which is how the shader decides
/// it can skip evaluating the per channel curves.
pub fn is_default(points: &[Point]) -> bool {
    if points.len() < 2 {
        return false;
    }
    let identity = points.iter().all(|p| (p.x - p.y).abs() <= 0.5);
    let first = points[0];
    let last = points[points.len() - 1];
    identity
        && first.x.abs() < 0.1
        && first.y.abs() < 0.1
        && (last.x - 255.0).abs() < 0.1
        && (last.y - 255.0).abs() < 0.1
}

fn hermite(x: f32, p1: Point, p2: Point, m1: f32, m2: f32) -> f32 {
    let dx = p2.x - p1.x;
    if dx <= 0.0 {
        return p1.y;
    }
    let t = (x - p1.x) / dx;
    let (t2, t3) = (t * t, t * t * t);
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * p1.y + h10 * m1 * dx + h01 * p2.y + h11 * m2 * dx
}

/// Monotone cubic interpolation through the curve's points.
///
/// Tangents are limited the Fritsch-Carlson way, so a curve that only rises
/// cannot dip between two of its own points and invert a tone.
pub fn apply(value: f32, points: &[Point]) -> f32 {
    let count = points.len();
    if count < 2 {
        return value;
    }
    let x = value * 255.0;
    if x <= points[0].x {
        return points[0].y / 255.0;
    }
    if x >= points[count - 1].x {
        return points[count - 1].y / 255.0;
    }

    for i in 0..count - 1 {
        let (p1, p2) = (points[i], points[i + 1]);
        if x > p2.x {
            continue;
        }
        let p0 = points[i.saturating_sub(1)];
        let p3 = points[(i + 2).min(count - 1)];

        let delta_before = (p1.y - p0.y) / (p1.x - p0.x).max(0.001);
        let delta_current = (p2.y - p1.y) / (p2.x - p1.x).max(0.001);
        let delta_after = (p3.y - p2.y) / (p3.x - p2.x).max(0.001);

        let mut m1 = if i == 0 {
            delta_current
        } else if delta_before * delta_current <= 0.0 {
            0.0
        } else {
            (delta_before + delta_current) / 2.0
        };
        let mut m2 = if i + 1 == count - 1 {
            delta_current
        } else if delta_current * delta_after <= 0.0 {
            0.0
        } else {
            (delta_current + delta_after) / 2.0
        };

        if delta_current != 0.0 {
            let alpha = m1 / delta_current;
            let beta = m2 / delta_current;
            if alpha * alpha + beta * beta > 9.0 {
                let tau = 3.0 / (alpha * alpha + beta * beta).sqrt();
                m1 *= tau;
                m2 *= tau;
            }
        }

        return (hermite(x, p1, p2, m1, m2) / 255.0).clamp(0.0, 1.0);
    }

    points[count - 1].y / 255.0
}

/// The luma curve across all three channels, then the per channel curves.
pub fn apply_all(
    color: [f32; 3],
    luma: &[Point],
    red: &[Point],
    green: &[Point],
    blue: &[Point],
) -> [f32; 3] {
    let curved = [
        apply(color[0], luma),
        apply(color[1], luma),
        apply(color[2], luma),
    ];

    if is_default(red) && is_default(green) && is_default(blue) {
        return curved;
    }

    [
        apply(curved[0], red),
        apply(curved[1], green),
        apply(curved[2], blue),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_probe;

    fn point(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    /// A curve that lifts the shadows without moving the endpoints.
    fn shadow_lift() -> Vec<Point> {
        vec![point(0.0, 0.0), point(64.0, 96.0), point(255.0, 255.0)]
    }

    /// Off default by the smallest amount `is_default` still notices, so it
    /// selects the per channel path while barely changing a colour.
    fn barely_off_default() -> Vec<Point> {
        vec![point(0.0, 0.0), point(128.0, 129.0), point(255.0, 255.0)]
    }

    #[test]
    fn an_identity_curve_changes_nothing() {
        for v in [0.0f32, 0.05, 0.25, 0.5, 0.75, 1.0] {
            assert!((apply(v, &IDENTITY) - v).abs() < 1e-3, "{v} moved");
        }
    }

    /// The bug this module was written for. The luma curve used to be a
    /// luminance target when any RGB curve was off default and a per channel
    /// curve otherwise, so nudging one red point changed what it did to
    /// everything.
    #[test]
    fn the_luma_curve_does_the_same_thing_whatever_the_rgb_curves_are_doing() {
        let luma = shadow_lift();
        let nudged = barely_off_default();
        assert!(!is_default(&nudged), "the fixture no longer takes the path");

        for colour in [
            [0.2f32, 0.3, 0.5],
            [0.05, 0.05, 0.05],
            [0.8, 0.4, 0.1],
            [0.5, 0.5, 0.5],
        ] {
            let alone = apply_all(colour, &luma, &IDENTITY, &IDENTITY, &IDENTITY);
            let alongside = apply_all(colour, &luma, &nudged, &nudged, &nudged);
            for c in 0..3 {
                assert!(
                    (alone[c] - alongside[c]).abs() < 0.01,
                    "{colour:?} channel {c}: {} alone against {} with a nudged RGB curve",
                    alone[c],
                    alongside[c]
                );
            }
        }
    }

    /// Holding luminance to the luma curve's target meant an RGB curve could
    /// only tint. Lifting the shadows with the blue curve has to lighten them.
    #[test]
    fn an_rgb_curve_can_change_brightness_and_not_only_colour() {
        let shadow = [0.10f32, 0.12, 0.15];
        let lifted = apply_all(shadow, &IDENTITY, &IDENTITY, &IDENTITY, &shadow_lift());

        assert!(
            lifted[2] > shadow[2] + 0.05,
            "the blue curve did not lift blue: {} to {}",
            shadow[2],
            lifted[2]
        );
        assert!(
            display_luma(lifted) > display_luma(shadow) + 0.005,
            "the blue lift did not reach luminance: {} to {}",
            display_luma(shadow),
            display_luma(lifted)
        );
    }

    #[test]
    fn a_rising_curve_never_dips() {
        let steep = vec![
            point(0.0, 0.0),
            point(32.0, 8.0),
            point(200.0, 250.0),
            point(255.0, 255.0),
        ];
        let mut previous = -1.0f32;
        for step in 0..=512 {
            let current = apply(step as f32 / 512.0, &steep);
            assert!(
                current >= previous - 1e-6,
                "curve fell at {}: {previous} to {current}",
                step as f32 / 512.0
            );
            previous = current;
        }
    }

    #[test]
    fn a_curve_stays_inside_the_display_range() {
        let overshooting = vec![
            point(0.0, 0.0),
            point(20.0, 250.0),
            point(235.0, 5.0),
            point(255.0, 255.0),
        ];
        for step in 0..=512 {
            let v = apply(step as f32 / 512.0, &overshooting);
            assert!((0.0..=1.0).contains(&v), "curve left the range at {v}");
        }
    }

    /// The two semantics cannot come back without this failing.
    #[test]
    fn the_shader_has_one_curve_semantic() {
        let body = shader_probe::fn_body("apply_all_curves");
        assert!(
            !body.contains("luma_target"),
            "apply_all_curves reconstructs luminance again: {body:?}"
        );
        assert!(
            body.contains("apply_curve(color.r, luma_curve"),
            "apply_all_curves no longer applies the luma curve per channel"
        );
    }
}
