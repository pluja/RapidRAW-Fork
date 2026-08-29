//! Correlated colour temperature and chromatic adaptation for white balance.
//!
//! Temperature is expressed in mireds rather than kelvin so that a slider step
//! is perceptually even: 100 K at 3000 K is a large shift, at 10000 K it is
//! invisible.

use crate::color_space::{self, Mat3};

/// Mireds per unit of slider travel, so the full -100..100 range spans roughly
/// 2000 K to 30000 K from a daylight starting point.
pub const MIREDS_PER_STEP: f32 = 1.5;

/// Slider units of tint per 0.01 of CIE 1960 v, the axis perpendicular to the
/// locus along which green and magenta lie.
pub const TINT_V_PER_STEP: f32 = 0.0004;

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
    let (tx, ty) = xy_from_uv(u, v + tint * TINT_V_PER_STEP);
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn rejects_degenerate_coefficients() {
        let xyz2cam = color_space::invert(&color_space::PROPHOTO_TO_XYZ_D50).unwrap();
        assert!(as_shot_white_xyz(xyz2cam, [0.0, 1.0, 1.0, 1.0]).is_none());
        assert!(as_shot_white_xyz(xyz2cam, [f32::NAN, 1.0, 1.0, 1.0]).is_none());
    }
}
