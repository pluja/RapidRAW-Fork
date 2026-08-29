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
    /// Mirror the band selection constants in shader.wgsl.
    const SHADER_BAND_CHROMA_FLOOR: f32 = 0.02;
    const SHADER_BAND_SOFTNESS: f32 = 0.30;

    fn to_working(srgb: [f32; 3]) -> [f32; 3] {
        color_space::apply(
            &color_space::invert(&color_space::prophoto_to_srgb()).unwrap(),
            srgb,
        )
    }

    fn hue_distance(a: f32, b: f32) -> f32 {
        let d = (a - b).abs() % 360.0;
        d.min(360.0 - d)
    }

    /// Reproduces the shader's band selection: each band's share of a colour,
    /// measured by projecting the chroma vector onto the band's direction.
    fn band_shares(lab: [f32; 3], softness: f32) -> [f32; 8] {
        let chroma = (lab[1] * lab[1] + lab[2] * lab[2]).sqrt();
        let selector = [
            lab[1] / (chroma + SHADER_BAND_CHROMA_FLOOR),
            lab[2] / (chroma + SHADER_BAND_CHROMA_FLOOR),
        ];
        let mut weights = [0.0f32; 8];
        let mut total = 0.0;
        for i in 0..8 {
            let direction = BAND_CENTERS[i].to_radians();
            let alignment = selector[0] * direction.cos() + selector[1] * direction.sin();
            weights[i] = (alignment / softness).exp();
            total += weights[i];
        }
        let mut shares = [0.0f32; 8];
        for i in 0..8 {
            shares[i] = ((weights[i] / total - 0.125) / 0.875).max(0.0);
        }
        shares
    }

    fn band_luminance(rgb: [f32; 3], band: usize, amount: f32) -> f32 {
        let lab = oklab_from_prophoto(rgb);
        lab[0] * (1.0 + amount * band_shares(lab, SHADER_BAND_SOFTNESS)[band])
    }

    /// The band weights the shader replaced, for comparison. Selecting on hue
    /// angle is what let sensor noise reach the image.
    fn hue_angle_luminance(rgb: [f32; 3], band: usize, amount: f32, authority_high: f32) -> f32 {
        let lch = oklch_from_oklab(oklab_from_prophoto(rgb));
        let hue = lch[2].to_degrees().rem_euclid(360.0);
        let mut weights = [0.0f32; 8];
        let mut total = 0.0;
        for i in 0..8 {
            let reach = (1.0 - hue_distance(hue, BAND_CENTERS[i]) / 75.0).max(0.0);
            weights[i] = reach * reach * (3.0 - 2.0 * reach);
            total += weights[i];
        }
        let t = (lch[1] / authority_high).clamp(0.0, 1.0);
        let authority = t * t * (3.0 - 2.0 * t);
        lch[0] * (1.0 + amount * (weights[band] / total) * authority)
    }

    /// A real sky is far paler than a saturated blue: an X-S20 frame measures
    /// around 0.045 chroma at the top and 0.011 at the horizon. A band has to
    /// claim it anyway, because it is still unmistakably blue to look at.
    #[test]
    fn a_pale_sky_is_claimed_by_its_band() {
        let upper = to_working([0.42, 0.58, 0.75]);
        let share = band_shares(oklab_from_prophoto(upper), SHADER_BAND_SOFTNESS)[5];
        assert!(
            share > 0.30,
            "a pale sky took only {:.0}% of its band",
            share * 100.0
        );

        let horizon = to_working([0.72, 0.78, 0.82]);
        let faint = band_shares(oklab_from_prophoto(horizon), SHADER_BAND_SOFTNESS)[5];
        assert!(
            faint > 0.10 && faint < share,
            "the near-neutral horizon took {:.0}%",
            faint * 100.0
        );
    }

    /// A sky is smooth to the eye and noisy per pixel, so what the band lets
    /// through has to stay well under a display step.
    #[test]
    fn a_pale_sky_does_not_carry_its_noise_into_lightness() {
        let upper = to_working([0.42, 0.58, 0.75]);
        let noisy = to_working([0.425, 0.577, 0.756]);
        let base = oklab_from_prophoto(upper)[0];
        let wobble =
            (band_luminance(upper, 5, -0.99) - band_luminance(noisy, 5, -0.99)).abs() / base;
        assert!(
            wobble < 0.015,
            "a noise step moved lightness by {:.2}%",
            wobble * 100.0
        );
    }

    /// A colour no band owns divides evenly between all eight, so measuring
    /// each share against an even one leaves neutrals alone with no threshold
    /// to cross. The bound is a fraction of a display step rather than zero,
    /// since claiming pale colours firmly necessarily lets a trace through.
    #[test]
    fn neutrals_are_untouched_by_a_band() {
        for level in [0.15f32, 0.45, 0.8] {
            let grey = [level, level, level];
            let base = oklab_from_prophoto(grey)[0];
            let moved = (base - band_luminance(grey, 5, -0.99)).abs() / base;
            assert!(
                moved < 0.003,
                "grey at {level} moved by {:.4}%, which would show as banding",
                moved * 100.0
            );
        }
    }

    #[test]
    fn band_luminance_still_reaches_saturated_colour() {
        let vivid = to_working([0.10, 0.30, 0.85]);
        let base = oklab_from_prophoto(vivid)[0];
        let lifted = band_luminance(vivid, 5, -0.93);
        assert!(
            lifted < base * 0.80,
            "a vivid blue should respond strongly: {base} to {lifted}"
        );
    }

    #[test]
    fn a_band_does_not_reach_across_the_wheel() {
        // Setting blue must leave an orange essentially alone.
        let orange = to_working([0.85, 0.45, 0.15]);
        let base = oklab_from_prophoto(orange)[0];
        let moved = (base - band_luminance(orange, 5, -0.93)).abs() / base;
        assert!(moved < 0.02, "blue reached orange by {:.2}%", moved * 100.0);
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
