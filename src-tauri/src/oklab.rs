//! Oklab and its cylindrical form, as the space the creative colour controls
//! operate in.
//!
//! Scaling colour toward its own luminance in linear RGB, which is what the
//! pipeline did, moves along a line in a space where hue is not preserved:
//! blues drift purple and oranges drift yellow as saturation rises. Oklab
//! exists to fix exactly the hue non-linearity that CIELAB suffers from, so a
//! chroma change at constant hue angle stays the colour the user was looking at.
//!
//! Chroma here is absolute rather than normalised against a gamut boundary,
//! which is why this is Oklch and not Okhsl: normalising would tie the
//! behaviour of every control back to a display gamut, the coupling the wide
//! working space was introduced to remove.

#![allow(dead_code)]

use crate::color_space::{self, Mat3};
use std::sync::OnceLock;

/// XYZ (D65) to the cone-like LMS basis Oklab is built on.
const XYZ_D65_TO_LMS: Mat3 = [
    [0.8189330101, 0.3618667424, -0.1288597137],
    [0.0329845436, 0.9293118715, 0.0361456387],
    [0.0482003018, 0.2643662691, 0.6338517070],
];

/// Non-linear LMS to Oklab.
const LMS_TO_OKLAB: Mat3 = [
    [0.2104542553, 0.7936177850, -0.0040720468],
    [1.9779984951, -2.4285922050, 0.4505937099],
    [0.0259040371, 0.7827717662, -0.8086757660],
];

const WHITE_D50: [f32; 3] = [0.96422, 1.00000, 0.82521];
const WHITE_D65: [f32; 3] = [0.95047, 1.00000, 1.08883];

/// Working space straight to the LMS basis, so a conversion costs one matrix
/// rather than a chain through XYZ.
pub fn prophoto_to_lms() -> Mat3 {
    let adapt = color_space::bradford_adaptation(WHITE_D50, WHITE_D65);
    color_space::multiply(
        &XYZ_D65_TO_LMS,
        &color_space::multiply(&adapt, &color_space::PROPHOTO_TO_XYZ_D50),
    )
}

pub fn lms_to_prophoto() -> Mat3 {
    color_space::invert(&prophoto_to_lms()).expect("the LMS basis is invertible")
}

pub fn oklab_to_lms() -> Mat3 {
    color_space::invert(&LMS_TO_OKLAB).expect("the Oklab basis is invertible")
}

pub fn lms_to_oklab() -> Mat3 {
    LMS_TO_OKLAB
}

fn cached(slot: &'static OnceLock<Mat3>, build: fn() -> Mat3) -> &'static Mat3 {
    slot.get_or_init(build)
}

pub fn prophoto_to_lms_cached() -> &'static Mat3 {
    static CACHED: OnceLock<Mat3> = OnceLock::new();
    cached(&CACHED, prophoto_to_lms)
}

pub fn lms_to_prophoto_cached() -> &'static Mat3 {
    static CACHED: OnceLock<Mat3> = OnceLock::new();
    cached(&CACHED, lms_to_prophoto)
}

/// Cube root that keeps the sign of its argument.
///
/// A wide working space carries colours whose channels go slightly negative,
/// and a plain power of a negative base is not a number.
#[inline]
fn signed_cbrt(v: f32) -> f32 {
    v.signum() * v.abs().cbrt()
}

pub fn oklab_from_prophoto(rgb: [f32; 3]) -> [f32; 3] {
    let lms = color_space::apply(prophoto_to_lms_cached(), rgb);
    let nonlinear = [
        signed_cbrt(lms[0]),
        signed_cbrt(lms[1]),
        signed_cbrt(lms[2]),
    ];
    color_space::apply(&LMS_TO_OKLAB, nonlinear)
}

pub fn prophoto_from_oklab(lab: [f32; 3]) -> [f32; 3] {
    let nonlinear = color_space::apply(&oklab_to_lms(), lab);
    let lms = [
        nonlinear[0] * nonlinear[0] * nonlinear[0],
        nonlinear[1] * nonlinear[1] * nonlinear[1],
        nonlinear[2] * nonlinear[2] * nonlinear[2],
    ];
    color_space::apply(lms_to_prophoto_cached(), lms)
}

/// Lightness, chroma and hue in radians.
pub fn oklch_from_oklab(lab: [f32; 3]) -> [f32; 3] {
    [
        lab[0],
        (lab[1] * lab[1] + lab[2] * lab[2]).sqrt(),
        lab[2].atan2(lab[1]),
    ]
}

pub fn oklab_from_oklch(lch: [f32; 3]) -> [f32; 3] {
    [lch[0], lch[1] * lch[2].cos(), lch[1] * lch[2].sin()]
}

/// Hue angle in degrees of a colour given in linear sRGB, which is how the
/// band centres for the eight-way colour panel are derived.
pub fn oklch_hue_degrees_of_srgb(srgb: [f32; 3]) -> f32 {
    let prophoto = color_space::apply(
        &color_space::invert(&color_space::prophoto_to_srgb()).unwrap(),
        srgb,
    );
    let lch = oklch_from_oklab(oklab_from_prophoto(prophoto));
    lch[2].to_degrees().rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_matrix(actual: Mat3, expected: Mat3, tol: f32, what: &str) {
        for r in 0..3 {
            for c in 0..3 {
                assert!(
                    (actual[r][c] - expected[r][c]).abs() < tol,
                    "{what}[{r}][{c}]: {} vs shader {}",
                    actual[r][c],
                    expected[r][c]
                );
            }
        }
    }

    /// shader.wgsl carries these as literals. Drift would change every colour
    /// the creative controls touch.
    #[test]
    fn shader_matrices_match() {
        const SHADER_PP_TO_LMS: Mat3 = [
            [0.71538717, 0.35280859, -0.06826405],
            [0.27443418, 0.66782898, 0.05775598],
            [0.10983816, 0.18630311, 0.70419478],
        ];
        const SHADER_LMS_TO_PP: Mat3 = [
            [1.73857641, -0.98809987, 0.24957718],
            [-0.70716941, 1.93436372, -0.22720321],
            [-0.08408777, -0.35763812, 1.44124269],
        ];
        const SHADER_OKLAB_TO_LMS: Mat3 = [
            [1.00000000, 0.39633778, 0.21580376],
            [1.00000000, -0.10556135, -0.06385417],
            [1.00000000, -0.08948418, -1.29148555],
        ];
        assert_matrix(prophoto_to_lms(), SHADER_PP_TO_LMS, 1e-6, "PP_TO_LMS");
        assert_matrix(lms_to_prophoto(), SHADER_LMS_TO_PP, 1e-6, "LMS_TO_PP");
        assert_matrix(oklab_to_lms(), SHADER_OKLAB_TO_LMS, 1e-6, "OKLAB_TO_LMS");
    }

    /// The shader selects bands by these angles; they are the hues of the
    /// colours each band is named for.
    #[test]
    fn shader_band_centres_match_their_colours() {
        const SHADER_CENTERS: [f32; 8] =
            [29.23, 67.93, 109.78, 142.51, 194.80, 264.06, 311.99, 328.36];
        let colours = [
            [1.0, 0.0, 0.0],
            [1.0, 0.35, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.5, 0.0, 1.0],
            [1.0, 0.0, 1.0],
        ];
        for (i, colour) in colours.iter().enumerate() {
            let hue = oklch_hue_degrees_of_srgb(*colour);
            assert!(
                (hue - SHADER_CENTERS[i]).abs() < 0.05,
                "band {i} centre is {hue} but the shader uses {}",
                SHADER_CENTERS[i]
            );
        }
    }

    /// Skin holding one hue across tones is what makes the vibrance guard
    /// meaningful rather than arbitrary.
    #[test]
    fn skin_holds_its_hue_across_tones() {
        const SHADER_SKIN_HUE: f32 = 55.0;
        for skin in [[0.85, 0.66, 0.55], [0.62, 0.44, 0.34], [0.33, 0.21, 0.15]] {
            let hue = oklch_hue_degrees_of_srgb(skin);
            assert!(
                (hue - SHADER_SKIN_HUE).abs() < 3.0,
                "skin tone {skin:?} sits at {hue}, not near {SHADER_SKIN_HUE}"
            );
        }
    }

    const BAND_CENTERS: [f32; 8] = [29.23, 67.93, 109.78, 142.51, 194.80, 264.06, 311.99, 328.36];
    const BAND_FALLOFF_DEG: f32 = 75.0;
    /// Mirrors the authority constants in shader.wgsl.
    const SHADER_BAND_AUTHORITY: (f32, f32) = (0.01, 0.06);
    const SHADER_LUMA_AUTHORITY: (f32, f32) = (0.0, 0.30);

    fn hue_distance(a: f32, b: f32) -> f32 {
        let d = (a - b).abs() % 360.0;
        d.min(360.0 - d)
    }

    fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Reproduces the shader's band luminance path, so the noise behaviour can
    /// be measured here rather than only seen in a sky.
    fn band_luminance(rgb: [f32; 3], band: usize, amount: f32, luma_authority: (f32, f32)) -> f32 {
        let lch = oklch_from_oklab(oklab_from_prophoto(rgb));
        let (l, chroma) = (lch[0], lch[1]);
        let hue = lch[2].to_degrees().rem_euclid(360.0);

        let mut weights = [0.0f32; 8];
        let mut total = 0.0;
        for i in 0..8 {
            let reach = (1.0 - hue_distance(hue, BAND_CENTERS[i]) / BAND_FALLOFF_DEG).max(0.0);
            weights[i] = reach * reach * (3.0 - 2.0 * reach);
            total += weights[i];
        }
        let share = if total > 1e-6 {
            weights[band] / total
        } else {
            0.0
        };
        let authority = smoothstep(luma_authority.0, luma_authority.1, chroma);
        l * (1.0 + amount * share * authority)
    }

    /// A sky is smooth to the eye but noisy per pixel, and its hue is the
    /// arctangent of two small noisy numbers. Band influence has to fade with
    /// chroma or that noise is amplified straight into luminance.
    #[test]
    fn band_luminance_does_not_amplify_noise_in_low_chroma() {
        // A desaturated sky blue, and the same pixel a plausible noise step away.
        let sky = [0.32, 0.44, 0.62];
        let noisy = [0.325, 0.437, 0.628];

        let narrow_gate = (0.0, 0.02);
        let spread = (band_luminance(sky, 5, -0.24, narrow_gate)
            - band_luminance(noisy, 5, -0.24, narrow_gate))
        .abs();

        let ramped = SHADER_LUMA_AUTHORITY;
        let ramped_spread =
            (band_luminance(sky, 5, -0.24, ramped) - band_luminance(noisy, 5, -0.24, ramped)).abs();

        assert!(
            ramped_spread < spread * 0.35,
            "ramping authority across chroma should suppress the noise it lets through: \
             narrow gate spread {spread:.6}, ramped {ramped_spread:.6}"
        );
    }

    /// Hue and saturation reach full authority sooner than luminance does,
    /// which is the balance the previous HSV implementation also struck.
    #[test]
    fn hue_authority_arrives_before_luminance_authority() {
        let sky_chroma = 0.05;
        let hue_share = smoothstep(SHADER_BAND_AUTHORITY.0, SHADER_BAND_AUTHORITY.1, sky_chroma);
        let luma_share = smoothstep(SHADER_LUMA_AUTHORITY.0, SHADER_LUMA_AUTHORITY.1, sky_chroma);
        assert!(
            hue_share > luma_share * 3.0,
            "hue {hue_share:.3} should outpace luminance {luma_share:.3} at sky chroma"
        );
        assert!(
            luma_share < 0.15,
            "luminance authority in a sky was {luma_share:.3}"
        );
    }

    #[test]
    fn band_luminance_still_reaches_saturated_colour() {
        let vivid = [0.05, 0.12, 0.72];
        let plain = band_luminance(vivid, 5, 0.0, SHADER_LUMA_AUTHORITY);
        let lifted = band_luminance(vivid, 5, -0.24, SHADER_LUMA_AUTHORITY);
        assert!(
            lifted < plain * 0.93,
            "a saturated blue should still respond: {plain} to {lifted}"
        );
    }

    #[test]
    fn working_space_white_is_oklab_white() {
        let lab = oklab_from_prophoto([1.0, 1.0, 1.0]);
        assert!((lab[0] - 1.0).abs() < 1e-3, "L was {}", lab[0]);
        assert!(lab[1].abs() < 1e-3, "a was {}", lab[1]);
        assert!(lab[2].abs() < 1e-3, "b was {}", lab[2]);
    }

    #[test]
    fn black_is_oklab_black() {
        let lab = oklab_from_prophoto([0.0, 0.0, 0.0]);
        for c in lab {
            assert!(c.abs() < 1e-6, "black produced {lab:?}");
        }
    }

    #[test]
    fn conversion_round_trips() {
        for rgb in [
            [0.2, 0.5, 0.9],
            [0.9, 0.1, 0.05],
            [0.05, 0.05, 0.05],
            [1.0, 0.85, 0.7],
        ] {
            let back = prophoto_from_oklab(oklab_from_prophoto(rgb));
            for i in 0..3 {
                assert!(
                    (back[i] - rgb[i]).abs() < 1e-4,
                    "{rgb:?} round tripped to {back:?}"
                );
            }
        }
    }

    #[test]
    fn negative_channels_survive_the_round_trip() {
        // Wide-gamut data reaches these controls with channels below zero.
        let rgb = [-0.04, 0.6, 0.35];
        let back = prophoto_from_oklab(oklab_from_prophoto(rgb));
        for i in 0..3 {
            assert!(
                back[i].is_finite() && (back[i] - rgb[i]).abs() < 1e-4,
                "{rgb:?} round tripped to {back:?}"
            );
        }
    }

    #[test]
    fn greys_carry_no_chroma() {
        for level in [0.05f32, 0.2, 0.5, 0.9] {
            let lch = oklch_from_oklab(oklab_from_prophoto([level, level, level]));
            assert!(lch[1] < 1e-3, "grey at {level} had chroma {}", lch[1]);
        }
    }

    /// The point of the exercise: raising chroma must not move the hue the way
    /// scaling toward luminance in linear RGB does.
    #[test]
    fn raising_chroma_holds_hue_where_linear_rgb_does_not() {
        const LUMA: [f32; 3] = [0.2880402, 0.7118741, 0.0000857];

        for rgb in [[0.12, 0.20, 0.55], [0.55, 0.22, 0.08], [0.15, 0.42, 0.16]] {
            let before = oklch_from_oklab(oklab_from_prophoto(rgb));

            let boosted = oklab_from_oklch([before[0], before[1] * 1.6, before[2]]);
            let after = oklch_from_oklab(oklab_from_prophoto(prophoto_from_oklab(boosted)));
            let drift = (after[2] - before[2]).abs().to_degrees();
            assert!(
                drift < 0.5,
                "oklch saturation drifted hue by {drift} degrees"
            );

            let luma = rgb[0] * LUMA[0] + rgb[1] * LUMA[1] + rgb[2] * LUMA[2];
            let naive = [
                luma + (rgb[0] - luma) * 1.6,
                luma + (rgb[1] - luma) * 1.6,
                luma + (rgb[2] - luma) * 1.6,
            ];
            let naive_hue = oklch_from_oklab(oklab_from_prophoto(naive))[2];
            let naive_drift = (naive_hue - before[2]).abs().to_degrees();
            assert!(
                naive_drift > drift,
                "linear RGB drifted {naive_drift} degrees against oklch's {drift}"
            );
        }
    }
}
