//! Correlated colour temperature and chromatic adaptation for white balance.
//!
//! Temperature is expressed in mireds rather than kelvin so that a slider step
//! is perceptually even: 100 K at 3000 K is a large shift, at 10000 K it is
//! invisible.

// Wired into the pipeline by the shader and adjustment plumbing that follows;
// until then nothing outside the tests calls into here.
#![allow(dead_code)]

use crate::color_space::{self, Mat3};

/// Mireds per unit of slider travel, so the full -100..100 range spans roughly
/// 2900 K to 20000 K from the working space white.
pub const MIREDS_PER_STEP: f32 = 1.5;

/// Divisors the adjustment plumbing applies before values reach the shader, so
/// the shader's own constants are these multiples of the per-slider ones.
pub const TEMPERATURE_SCALE: f32 = 25.0;
pub const TINT_SCALE: f32 = 100.0;

/// Temperature of the working space white, which is where the shot's own
/// neutral was pinned during develop and so where the sliders move from.
pub const ORIGIN_CCT: f32 = 5003.0;

/// Slider units of tint per 0.01 of CIE 1960 v, the axis perpendicular to the
/// locus along which green and magenta lie.
pub const TINT_V_PER_STEP: f32 = 0.0003;

/// Smallest Z the target illuminant may have.
///
/// The tint offset moves v with no regard for where the visible gamut ends. Past
/// x + y = 1 the target's Z turns negative, its Bradford blue cone crosses zero,
/// and the gain divided through it passes a pole and comes back negated. A warm
/// target with a green cast pulled out of it reaches that corner on the ordinary
/// sliders, so the target is held to a floor on Z instead. Tint saturates there
/// rather than exploding, since no greener light exists at that chromaticity.
///
/// Mirrors WB_TARGET_MIN_Z in shaders/shader.wgsl, which the tests pin against
/// the shader source rather than against a copy of it.
pub const TARGET_MIN_Z: f32 = 0.05;

/// Pulls a tinted target back to the visible gamut along the segment from the
/// untinted one. Z is affine along that segment, so the crossing is exact.
pub fn clamp_target_to_gamut(untinted: (f32, f32), tinted: (f32, f32)) -> (f32, f32) {
    let z_tinted = 1.0 - tinted.0 - tinted.1;
    if z_tinted >= TARGET_MIN_Z {
        return tinted;
    }
    let z_untinted = 1.0 - untinted.0 - untinted.1;
    let span = z_tinted - z_untinted;
    if span.abs() < 1e-9 {
        return untinted;
    }
    let t = ((TARGET_MIN_Z - z_untinted) / span).clamp(0.0, 1.0);
    (
        untinted.0 + t * (tinted.0 - untinted.0),
        untinted.1 + t * (tinted.1 - untinted.1),
    )
}

pub fn xyz_from_xy(x: f32, y: f32) -> [f32; 3] {
    if y.abs() < 1e-6 {
        return [0.0, 0.0, 0.0];
    }
    [x / y, 1.0, (1.0 - x - y) / y]
}

pub fn xy_from_xyz(xyz: [f32; 3]) -> (f32, f32) {
    let sum = xyz[0] + xyz[1] + xyz[2];
    if sum.abs() < 1e-9 {
        return (0.3127, 0.3290);
    }
    (xyz[0] / sum, xyz[1] / sum)
}

/// CIE 1960 uv, the space the tint axis is defined in.
pub fn uv_from_xy(x: f32, y: f32) -> (f32, f32) {
    let d = -2.0 * x + 12.0 * y + 3.0;
    if d.abs() < 1e-9 {
        return (0.0, 0.0);
    }
    (4.0 * x / d, 6.0 * y / d)
}

pub fn xy_from_uv(u: f32, v: f32) -> (f32, f32) {
    let d = 2.0 * u - 8.0 * v + 4.0;
    if d.abs() < 1e-9 {
        return (0.3127, 0.3290);
    }
    (3.0 * u / d, 2.0 * v / d)
}

/// McCamy's cubic approximation, accurate to about 2 K over 2000-12500 K.
pub fn cct_from_xy(x: f32, y: f32) -> f32 {
    let denom = 0.1858 - y;
    if denom.abs() < 1e-6 {
        return 6500.0;
    }
    let n = (x - 0.3320) / denom;
    (449.0 * n * n * n + 3525.0 * n * n + 6823.3 * n + 5520.33).clamp(1000.0, 50000.0)
}

/// Chromaticity of the illuminant at a given temperature.
///
/// Follows the DNG convention of the Planckian locus below 4000 K, where real
/// light sources are incandescent, and the CIE daylight locus above it.
pub fn xy_from_cct(cct: f32) -> (f32, f32) {
    let t = cct.clamp(1000.0, 50000.0);
    let inv = 1.0e3 / t;
    let inv2 = inv * inv;
    let inv3 = inv2 * inv;

    if t < 4000.0 {
        // Kim et al. cubic fit to the Planckian locus.
        let x = if t <= 2222.0 {
            -0.2661239 * inv3 - 0.2343589 * inv2 + 0.8776956 * inv + 0.179910
        } else {
            -3.0258469 * inv3 + 2.1070379 * inv2 + 0.2226347 * inv + 0.240390
        };
        let y = if t <= 2222.0 {
            -1.1063814 * x * x * x - 1.34811020 * x * x + 2.18555832 * x - 0.20219683
        } else {
            -0.9549476 * x * x * x - 1.37418593 * x * x + 2.09137015 * x - 0.16748867
        };
        (x, y)
    } else {
        let x = if t <= 7000.0 {
            -4.6070 * inv3 + 2.9678 * inv2 + 0.09911 * inv + 0.244063
        } else {
            -2.0064 * inv3 + 1.9018 * inv2 + 0.24748 * inv + 0.237040
        };
        let y = -3.000 * x * x + 2.870 * x - 0.275;
        (x, y)
    }
}

/// The illuminant the camera balanced against, recovered from its neutral.
///
/// A scene neutral sits at the reciprocal of the white balance coefficients in
/// camera space, so mapping that through the camera matrix gives the
/// illuminant the shot was taken under.
pub fn as_shot_white_xyz(xyz2cam: Mat3, wb_coeffs: [f32; 4]) -> Option<[f32; 3]> {
    let cam2xyz = color_space::invert(&xyz2cam)?;

    let neutral = [
        safe_reciprocal(wb_coeffs[0])?,
        safe_reciprocal(wb_coeffs[1])?,
        safe_reciprocal(wb_coeffs[2])?,
    ];
    let xyz = color_space::apply(&cam2xyz, neutral);
    if xyz[1].abs() < 1e-9 {
        return None;
    }
    Some([xyz[0] / xyz[1], 1.0, xyz[2] / xyz[1]])
}

fn safe_reciprocal(v: f32) -> Option<f32> {
    if v.is_finite() && v.abs() > 1e-6 {
        Some(1.0 / v)
    } else {
        None
    }
}

/// The illuminant the slider positions ask for, relative to the shot.
///
/// A higher target temperature means the scene is assumed to have been lit more
/// coolly than the correction applied, which is what warms the image, so
/// positive slider travel subtracts mireds.
pub fn target_white_xyz(as_shot_cct: f32, temperature: f32, tint: f32) -> [f32; 3] {
    let as_shot_mireds = 1.0e6 / as_shot_cct.clamp(1000.0, 50000.0);
    let target_mireds = (as_shot_mireds - temperature * MIREDS_PER_STEP).clamp(20.0, 1000.0);
    let target_cct = 1.0e6 / target_mireds;

    let (x, y) = xy_from_cct(target_cct);
    let (u, v) = uv_from_xy(x, y);
    // Green lies at higher v, and the target illuminant is divided out, so a
    // greener target is what leaves the image magenta.
    let (tx, ty) =
        clamp_target_to_gamut(xy_from_uv(u, v), xy_from_uv(u, v + tint * TINT_V_PER_STEP));
    xyz_from_xy(tx, ty)
}

/// The white balance adjustment as it applies to working-space pixels.
///
/// Dividing by a cooler assumed illuminant is what leaves an image warmer, so
/// the cone-space ratio runs from the target toward the shot.
pub fn adaptation_matrix(as_shot: [f32; 3], target: [f32; 3]) -> Option<Mat3> {
    let xyz_to_pp = color_space::invert(&color_space::PROPHOTO_TO_XYZ_D50)?;
    let adapt = color_space::bradford_adaptation(target, as_shot);
    Some(color_space::multiply(
        &xyz_to_pp,
        &color_space::multiply(&adapt, &color_space::PROPHOTO_TO_XYZ_D50),
    ))
}

/// Blends two illuminant-referenced camera matrices at a temperature.
///
/// Interpolation runs in mireds, following the DNG specification, so the blend
/// tracks perceived colour rather than crowding at the warm end.
fn blend_camera_matrices(warm: (f32, Mat3), cool: (f32, Mat3), cct: f32) -> Mat3 {
    let mired = 1.0e6 / cct.clamp(1000.0, 50000.0);
    let warm_mired = 1.0e6 / warm.0;
    let cool_mired = 1.0e6 / cool.0;

    let span = warm_mired - cool_mired;
    let g = if span.abs() < 1e-6 {
        1.0
    } else {
        ((mired - cool_mired) / span).clamp(0.0, 1.0)
    };

    let mut out = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = g * warm.1[i][j] + (1.0 - g) * cool.1[i][j];
        }
    }
    out
}

/// Resolves the camera matrix for the illuminant a frame was actually shot
/// under, given matrices measured at two reference illuminants.
///
/// The scene temperature is only knowable through the matrix, and the matrix
/// depends on the temperature, so the two are settled by iteration. A handful
/// of passes converges; the loop exits early once the estimate stops moving.
pub fn interpolate_camera_matrix(
    warm: (f32, Mat3),
    cool: (f32, Mat3),
    wb_coeffs: [f32; 4],
) -> Option<Mat3> {
    const MAX_PASSES: usize = 6;
    const SETTLED_KELVIN: f32 = 1.0;

    let mut cct = 5000.0f32;
    for _ in 0..MAX_PASSES {
        let matrix = blend_camera_matrices(warm, cool, cct);
        let white = as_shot_white_xyz(matrix, wb_coeffs)?;
        let (x, y) = xy_from_xyz(white);
        let next = cct_from_xy(x, y);

        if (next - cct).abs() < SETTLED_KELVIN {
            cct = next;
            break;
        }
        cct = next;
    }
    Some(blend_camera_matrices(warm, cool, cct))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_probe;

    fn assert_near(a: f32, b: f32, tol: f32, what: &str) {
        assert!((a - b).abs() < tol, "{what}: {a} vs {b} (tol {tol})");
    }

    #[test]
    fn daylight_locus_hits_known_illuminants() {
        let (x, y) = xy_from_cct(6504.0);
        assert_near(x, 0.3127, 2e-3, "D65 x");
        assert_near(y, 0.3290, 2e-3, "D65 y");

        let (x, y) = xy_from_cct(5003.0);
        assert_near(x, 0.3457, 3e-3, "D50 x");
        assert_near(y, 0.3585, 3e-3, "D50 y");
    }

    #[test]
    fn cct_round_trips_through_the_locus() {
        for cct in [2500.0, 3200.0, 4500.0, 5500.0, 6500.0, 9000.0] {
            let (x, y) = xy_from_cct(cct);
            let back = cct_from_xy(x, y);
            assert!(
                (back - cct).abs() / cct < 0.02,
                "{cct} K round tripped to {back} K"
            );
        }
    }

    #[test]
    fn uv_round_trips() {
        let (u, v) = uv_from_xy(0.3127, 0.3290);
        let (x, y) = xy_from_uv(u, v);
        assert_near(x, 0.3127, 1e-5, "x");
        assert_near(y, 0.3290, 1e-5, "y");
    }

    #[test]
    fn neutral_sliders_are_identity() {
        let as_shot = xyz_from_xy(0.3127, 0.3290);
        let target = target_white_xyz(cct_from_xy(0.3127, 0.3290), 0.0, 0.0);
        let m = adaptation_matrix(as_shot, target).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (m[i][j] - expected).abs() < 2e-3,
                    "[{i}][{j}] was {} with both sliders at zero",
                    m[i][j]
                );
            }
        }
    }

    #[test]
    fn positive_temperature_warms_the_image() {
        let as_shot_cct = 5500.0;
        let as_shot = xyz_from_xy(xy_from_cct(as_shot_cct).0, xy_from_cct(as_shot_cct).1);
        let grey = [0.5, 0.5, 0.5];

        let warm = color_space::apply(
            &adaptation_matrix(as_shot, target_white_xyz(as_shot_cct, 40.0, 0.0)).unwrap(),
            grey,
        );
        let cool = color_space::apply(
            &adaptation_matrix(as_shot, target_white_xyz(as_shot_cct, -40.0, 0.0)).unwrap(),
            grey,
        );

        let warm_ratio = warm[0] / warm[2];
        let cool_ratio = cool[0] / cool[2];
        assert!(
            warm_ratio > 1.0 && cool_ratio < 1.0,
            "positive temperature should raise red over blue: warm {warm_ratio}, cool {cool_ratio}"
        );
    }

    #[test]
    fn positive_tint_moves_toward_magenta() {
        let as_shot_cct = 5500.0;
        let (x, y) = xy_from_cct(as_shot_cct);
        let as_shot = xyz_from_xy(x, y);
        let grey = [0.5, 0.5, 0.5];

        let tinted = color_space::apply(
            &adaptation_matrix(as_shot, target_white_xyz(as_shot_cct, 0.0, 40.0)).unwrap(),
            grey,
        );
        let magenta = (tinted[0] + tinted[2]) * 0.5 - tinted[1];
        assert!(
            magenta > 0.0,
            "positive tint should lift red and blue over green: {magenta}"
        );
    }

    #[test]
    fn as_shot_white_recovers_the_illuminant() {
        // A camera whose matrix is the identity onto ProPhoto sees its own
        // neutral as D50.
        let xyz2cam = color_space::invert(&color_space::PROPHOTO_TO_XYZ_D50).unwrap();
        let white = as_shot_white_xyz(xyz2cam, [1.0, 1.0, 1.0, 1.0]).unwrap();
        let (x, y) = xy_from_xyz(white);
        assert_near(x, 0.3457, 2e-3, "recovered x");
        assert_near(y, 0.3585, 2e-3, "recovered y");
    }

    /// Fujifilm X-S20 ColorMatrix2, against coefficients read from a real
    /// frame, so a sign or scale error in the recovery shows up as a
    /// temperature no photograph would have been taken at.
    #[test]
    fn recovers_a_plausible_temperature_from_a_real_camera() {
        const X_S20_D65: Mat3 = [
            [1.2836, -0.5909, -0.1032],
            [-0.3087, 1.1132, 0.2236],
            [-0.0035, 0.0872, 0.5330],
        ];

        let overcast = as_shot_white_xyz(X_S20_D65, [1.8179, 1.0, 1.8808, 1.0]).unwrap();
        let (x, y) = xy_from_xyz(overcast);
        let cct = cct_from_xy(x, y);
        assert!(
            (4900.0..5400.0).contains(&cct),
            "overcast daylight recovered as {cct} K"
        );

        let tungsten = as_shot_white_xyz(X_S20_D65, [1.1, 1.0, 3.2, 1.0]).unwrap();
        let (x, y) = xy_from_xyz(tungsten);
        let cct = cct_from_xy(x, y);
        assert!(
            (2600.0..3100.0).contains(&cct),
            "incandescent recovered as {cct} K, expected near StdA at 2856 K"
        );
    }

    /// The cone response the shader carries as BRADFORD, and which
    /// color_space keeps privately. Held to the shader by
    /// `shader_cone_matrices_match`.
    const BRADFORD: Mat3 = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];

    /// Reproduces exactly what apply_white_balance does in shader.wgsl, so the
    /// two implementations cannot drift apart unnoticed. Takes slider units and
    /// scales them the way the adjustment plumbing does.
    fn shader_white_balance(color: [f32; 3], slider_temp: f32, slider_tint: f32) -> [f32; 3] {
        let temp = slider_temp / TEMPERATURE_SCALE;
        let tint = slider_tint / TINT_SCALE;

        let (ox, oy) = xy_from_cct(ORIGIN_CCT);
        let target_mireds =
            (1.0e6 / ORIGIN_CCT - temp * SHADER_MIREDS_PER_STEP).clamp(20.0, 1000.0);
        let (tx, ty) = xy_from_cct(1.0e6 / target_mireds);
        let (tu, tv) = uv_from_xy(tx, ty);
        let (fx, fy) = clamp_target_to_gamut(
            xy_from_uv(tu, tv),
            xy_from_uv(tu, tv + tint * SHADER_TINT_V_PER_STEP),
        );

        let origin_cone = color_space::apply(&BRADFORD, xyz_from_xy(ox, oy));
        let target_cone = color_space::apply(&BRADFORD, xyz_from_xy(fx, fy));

        let pp_to_cone = color_space::multiply(&BRADFORD, &color_space::PROPHOTO_TO_XYZ_D50);
        let cone_to_pp = color_space::multiply(
            &color_space::invert(&color_space::PROPHOTO_TO_XYZ_D50).unwrap(),
            &color_space::invert(&BRADFORD).unwrap(),
        );

        let cone = color_space::apply(&pp_to_cone, color);
        let scaled = [
            cone[0] * origin_cone[0] / target_cone[0],
            cone[1] * origin_cone[1] / target_cone[1],
            cone[2] * origin_cone[2] / target_cone[2],
        ];
        color_space::apply(&cone_to_pp, scaled)
    }

    #[test]
    fn the_shader_carries_the_gamut_floor_this_module_was_tuned_against() {
        let shader = shader_probe::f32_const("WB_TARGET_MIN_Z");
        assert!(
            (shader - TARGET_MIN_Z).abs() < 1e-9,
            "shader WB_TARGET_MIN_Z is {shader}, this module uses {TARGET_MIN_Z}"
        );
    }

    #[test]
    fn the_shader_white_balance_constants_match_this_module() {
        for (name, expected) in [
            ("WB_ORIGIN_CCT", ORIGIN_CCT),
            ("WB_MIREDS_PER_STEP", MIREDS_PER_STEP * TEMPERATURE_SCALE),
            ("WB_TINT_V_PER_STEP", TINT_V_PER_STEP * TINT_SCALE),
        ] {
            let shader = shader_probe::f32_const(name);
            assert!(
                (shader - expected).abs() <= expected.abs() * 1e-6,
                "shader {name} is {shader}, this module implies {expected}"
            );
        }
    }

    #[test]
    fn the_shader_applies_the_clamp_rather_than_merely_defining_it() {
        let body = shader_probe::fn_body("apply_white_balance");
        assert!(
            body.contains("wb_clamp_target_to_gamut"),
            "apply_white_balance does not call wb_clamp_target_to_gamut, so the \
             target can leave the visible gamut again"
        );
    }

    #[test]
    fn no_slider_position_can_invert_or_explode_a_channel() {
        // The pole this guards sat at temperature -100, tint +77: the target left
        // the gamut, its blue cone crossed zero, and the gain through it came back
        // negative. Sweeping every position is cheap, and the corner is easy to
        // miss by sampling.
        let grey = [0.18, 0.18, 0.18];
        let mut worst = 0.0f32;
        let mut worst_at = (0.0f32, 0.0f32);

        for ti in -100..=100 {
            for tn in -100..=100 {
                let (temp, tint) = (ti as f32, tn as f32);
                let out = shader_white_balance(grey, temp, tint);
                for (i, v) in out.iter().enumerate() {
                    assert!(
                        v.is_finite(),
                        "channel {i} was {v} at temperature {temp}, tint {tint}"
                    );
                    assert!(
                        *v > 0.0,
                        "channel {i} went to {v} at temperature {temp}, tint {tint}: \
                         a positive grey cannot balance to a negative channel"
                    );
                }
                let gain = out.iter().fold(0.0f32, |m, v| m.max(v / 0.18));
                if gain > worst {
                    worst = gain;
                    worst_at = (temp, tint);
                }
            }
        }

        assert!(
            worst < 20.0,
            "largest channel gain was {worst} at temperature {}, tint {}; the \
             gamut floor is meant to hold this near 11",
            worst_at.0,
            worst_at.1
        );
    }

    #[test]
    fn the_clamp_leaves_ordinary_edits_untouched() {
        // It may only bite in the corner it was added for. Anywhere else it would
        // be silently changing colour that was already correct.
        for (temp, tint) in [
            (0.0, 0.0),
            (-20.0, 20.0),
            (-40.0, 30.0),
            (20.0, -20.0),
            (-60.0, 50.0),
            (100.0, 100.0),
            (-100.0, 0.0),
            (-100.0, -100.0),
        ] {
            let t: f32 = temp / TEMPERATURE_SCALE;
            let tn: f32 = tint / TINT_SCALE;
            let mireds = (1.0e6 / ORIGIN_CCT - t * SHADER_MIREDS_PER_STEP).clamp(20.0, 1000.0);
            let (x, y) = xy_from_cct(1.0e6 / mireds);
            let (u, v) = uv_from_xy(x, y);
            let untinted = xy_from_uv(u, v);
            let tinted = xy_from_uv(u, v + tn * SHADER_TINT_V_PER_STEP);
            let clamped = clamp_target_to_gamut(untinted, tinted);
            assert!(
                (clamped.0 - tinted.0).abs() < 1e-7 && (clamped.1 - tinted.1).abs() < 1e-7,
                "temperature {temp}, tint {tint} was clamped from {tinted:?} to {clamped:?}"
            );
        }
    }

    #[test]
    fn shader_algorithm_is_identity_at_zero() {
        let grey = [0.42, 0.42, 0.42];
        let out = shader_white_balance(grey, 0.0, 0.0);
        for i in 0..3 {
            assert!(
                (out[i] - grey[i]).abs() < 1e-5,
                "channel {i} moved from {} to {} with both sliders at zero",
                grey[i],
                out[i]
            );
        }
    }

    #[test]
    fn shader_algorithm_matches_slider_directions() {
        let grey = [0.5, 0.5, 0.5];
        let warm = shader_white_balance(grey, 40.0, 0.0);
        let cool = shader_white_balance(grey, -40.0, 0.0);
        assert!(
            warm[0] / warm[2] > 1.0 && cool[0] / cool[2] < 1.0,
            "temperature direction wrong: warm {warm:?}, cool {cool:?}"
        );

        let magenta = shader_white_balance(grey, 0.0, 40.0);
        assert!(
            (magenta[0] + magenta[2]) * 0.5 > magenta[1],
            "tint direction wrong: {magenta:?}"
        );
    }

    const SHADER_MIREDS_PER_STEP: f32 = 37.5;
    const SHADER_TINT_V_PER_STEP: f32 = 0.03;

    /// The plumbing divides slider values before the shader sees them, so the
    /// shader's constants have to carry that factor.
    #[test]
    fn shader_step_constants_absorb_the_plumbing_scale() {
        assert!((SHADER_MIREDS_PER_STEP - MIREDS_PER_STEP * TEMPERATURE_SCALE).abs() < 1e-4);
        assert!((SHADER_TINT_V_PER_STEP - TINT_V_PER_STEP * TINT_SCALE).abs() < 1e-6);
    }

    #[test]
    fn full_slider_travel_spans_a_useful_range() {
        let origin_mireds = 1.0e6 / ORIGIN_CCT;
        let warm = 1.0e6 / (origin_mireds - 100.0 * MIREDS_PER_STEP);
        let cool = 1.0e6 / (origin_mireds + 100.0 * MIREDS_PER_STEP);
        assert!(
            warm > 15000.0 && (2500.0..3500.0).contains(&cool),
            "full travel reached {cool} K to {warm} K"
        );
    }

    /// The shader carries these as literals; drift would change every image.
    #[test]
    fn shader_cone_matrices_match() {
        let pp_to_cone = color_space::multiply(&BRADFORD, &color_space::PROPHOTO_TO_XYZ_D50);
        let cone_to_pp = color_space::invert(&pp_to_cone).unwrap();
        for (derived, name) in [
            (BRADFORD, "BRADFORD"),
            (pp_to_cone, "PP_TO_CONE"),
            (cone_to_pp, "CONE_TO_PP"),
        ] {
            let shader = shader_probe::mat3_const(name);
            for r in 0..3 {
                for c in 0..3 {
                    assert!(
                        (derived[r][c] - shader[r][c]).abs() < 1e-6,
                        "{name}[{r}][{c}]: {} vs shader {}",
                        derived[r][c],
                        shader[r][c]
                    );
                }
            }
        }
    }

    const X_S20_A: Mat3 = [
        [1.6344, -1.0648, 0.1184],
        [-0.2749, 1.0771, 0.2278],
        [0.0152, 0.0417, 0.6427],
    ];
    const X_S20_D65: Mat3 = [
        [1.2836, -0.5909, -0.1032],
        [-0.3087, 1.1132, 0.2236],
        [-0.0035, 0.0872, 0.5330],
    ];

    fn max_difference(a: Mat3, b: Mat3) -> f32 {
        let mut worst = 0.0f32;
        for r in 0..3 {
            for c in 0..3 {
                worst = worst.max((a[r][c] - b[r][c]).abs());
            }
        }
        worst
    }

    #[test]
    fn blending_at_a_reference_returns_that_matrix() {
        let warm = (2856.0, X_S20_A);
        let cool = (6504.0, X_S20_D65);
        assert!(max_difference(blend_camera_matrices(warm, cool, 2856.0), X_S20_A) < 1e-5);
        assert!(max_difference(blend_camera_matrices(warm, cool, 6504.0), X_S20_D65) < 1e-5);
    }

    #[test]
    fn blending_clamps_outside_the_reference_range() {
        let warm = (2856.0, X_S20_A);
        let cool = (6504.0, X_S20_D65);
        assert!(max_difference(blend_camera_matrices(warm, cool, 1200.0), X_S20_A) < 1e-5);
        assert!(max_difference(blend_camera_matrices(warm, cool, 20000.0), X_S20_D65) < 1e-5);
    }

    /// Interpolation is circular, since the temperature is read through the very
    /// matrix it selects. These check the loop settles somewhere physical.
    #[test]
    fn interpolation_converges_toward_the_lit_reference() {
        let warm = (2856.0, X_S20_A);
        let cool = (6504.0, X_S20_D65);

        let daylight = interpolate_camera_matrix(warm, cool, [1.8179, 1.0, 1.8808, 1.0]).unwrap();
        let (x, y) = xy_from_xyz(as_shot_white_xyz(daylight, [1.8179, 1.0, 1.8808, 1.0]).unwrap());
        let cct = cct_from_xy(x, y);
        assert!(
            (4500.0..6000.0).contains(&cct),
            "overcast frame settled at {cct} K"
        );

        let tungsten = interpolate_camera_matrix(warm, cool, [1.1, 1.0, 3.2, 1.0]).unwrap();
        let (x, y) = xy_from_xyz(as_shot_white_xyz(tungsten, [1.1, 1.0, 3.2, 1.0]).unwrap());
        let warm_cct = cct_from_xy(x, y);
        assert!(
            warm_cct < 3600.0,
            "incandescent frame settled at {warm_cct} K"
        );

        assert!(
            max_difference(tungsten, X_S20_A) < max_difference(daylight, X_S20_A),
            "the warmer frame should resolve nearer the incandescent reference"
        );
    }

    #[test]
    fn interpolation_differs_from_using_one_reference_alone() {
        let tungsten =
            interpolate_camera_matrix((2856.0, X_S20_A), (6504.0, X_S20_D65), [1.1, 1.0, 3.2, 1.0])
                .unwrap();
        assert!(
            max_difference(tungsten, X_S20_D65) > 0.05,
            "interpolation collapsed onto the daylight matrix"
        );
    }

    #[test]
    fn rejects_degenerate_coefficients() {
        let xyz2cam = color_space::invert(&color_space::PROPHOTO_TO_XYZ_D50).unwrap();
        assert!(as_shot_white_xyz(xyz2cam, [0.0, 1.0, 1.0, 1.0]).is_none());
        assert!(as_shot_white_xyz(xyz2cam, [f32::NAN, 1.0, 1.0, 1.0]).is_none());
    }
}
