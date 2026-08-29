//! Linear ProPhoto RGB (ROMM, D50) working space.
//!
//! Chosen over sRGB because a camera sensor records well outside the sRGB gamut,
//! and over Rec.2020 because the DNG spec defines DCP ProfileHueSatMap and
//! ProfileLookTable against ProPhoto primaries; keeping the pipeline there lets
//! camera profiles apply without a round trip.

use std::sync::OnceLock;

pub type Mat3 = [[f32; 3]; 3];

/// Rec.709 luminance weights, for operations in the sRGB output space.
const SRGB_LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// ROMM RGB (ISO 22028-2) primaries, D50 adapted.
pub const PROPHOTO_TO_XYZ_D50: Mat3 = [
    [0.7976749, 0.1351917, 0.0313534],
    [0.2880402, 0.7118741, 0.0000857],
    [0.0000000, 0.0000000, 0.8252100],
];

pub const XYZ_D65_TO_SRGB: Mat3 = [
    [3.2404542, -1.5371385, -0.4985314],
    [-0.9692660, 1.8760108, 0.0415560],
    [0.0556434, -0.2040259, 1.0572252],
];

const BRADFORD: Mat3 = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
];

const WHITE_D50: [f32; 3] = [0.96422, 1.00000, 0.82521];
const WHITE_D65: [f32; 3] = [0.95047, 1.00000, 1.08883];

/// Relative luminance weights for ProPhoto primaries.
///
/// Replaces the Rec.709 weights that are only correct for an sRGB working space.
/// Consumed by the shader as a literal rather than from here, so this is the
/// reference the shader constant is asserted against.
#[allow(dead_code)]
pub const PROPHOTO_LUMA: [f32; 3] = [
    PROPHOTO_TO_XYZ_D50[1][0],
    PROPHOTO_TO_XYZ_D50[1][1],
    PROPHOTO_TO_XYZ_D50[1][2],
];

pub fn multiply(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

pub fn apply(m: &Mat3, v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Inverts a 3x3 matrix, returning None when it is too ill-conditioned to trust.
///
/// Arithmetic is done in f64 because an f32 determinant of a camera matrix
/// carries roundoff on the order of 1e-7, which no absolute threshold can
/// distinguish from genuine near-singularity. The test is relative: the
/// determinant is compared against the product of the row 1-norms, which for
/// real camera matrices lands near 0.5 and for a rank-deficient one near zero.
pub fn invert(m: &Mat3) -> Option<Mat3> {
    let d = m.map(|row| row.map(f64::from));

    let det = d[0][0] * (d[1][1] * d[2][2] - d[1][2] * d[2][1])
        - d[0][1] * (d[1][0] * d[2][2] - d[1][2] * d[2][0])
        + d[0][2] * (d[1][0] * d[2][1] - d[1][1] * d[2][0]);

    let norm_product: f64 = d
        .iter()
        .map(|row| row.iter().map(|v| v.abs()).sum::<f64>())
        .product();

    if !det.is_finite() || norm_product == 0.0 || det.abs() / norm_product < 1e-4 {
        return None;
    }
    let inv_det = 1.0 / det;

    let mut out = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let (r0, r1) = ((j + 1) % 3, (j + 2) % 3);
            let (c0, c1) = ((i + 1) % 3, (i + 2) % 3);
            let cofactor = d[r0][c0] * d[r1][c1] - d[r0][c1] * d[r1][c0];
            out[i][j] = (cofactor * inv_det) as f32;
        }
    }
    Some(out)
}

/// Scales each row to sum to 1, pinning the neutral axis so a white-balanced
/// camera neutral maps to (1, 1, 1) in the destination space.
///
/// This is the dcraw/rawler approximation, not a true chromatic adaptation of
/// the camera matrix: off-neutral colours still differ from an illuminant
/// interpolated ColorMatrix/ForwardMatrix pipeline.
///
/// Returns None on a non-positive row sum, which would otherwise flip that
/// row's sign or leave it unscaled.
fn normalize_rows(m: Mat3) -> Option<Mat3> {
    let mut out = m;
    for row in out.iter_mut() {
        let sum: f32 = row.iter().sum();
        if !(sum > 1e-6) {
            return None;
        }
        for c in row.iter_mut() {
            *c /= sum;
        }
    }
    Some(out)
}

pub fn bradford_adaptation(src_white: [f32; 3], dst_white: [f32; 3]) -> Mat3 {
    let src_cone = apply(&BRADFORD, src_white);
    let dst_cone = apply(&BRADFORD, dst_white);

    let scale: Mat3 = [
        [dst_cone[0] / src_cone[0], 0.0, 0.0],
        [0.0, dst_cone[1] / src_cone[1], 0.0],
        [0.0, 0.0, dst_cone[2] / src_cone[2]],
    ];

    let bradford_inv = invert(&BRADFORD).expect("Bradford cone response matrix is invertible");
    multiply(&bradford_inv, &multiply(&scale, &BRADFORD))
}

/// Builds the unclipped camera-native to linear ProPhoto transform.
///
/// `xyz2cam` is the camera's DNG colour matrix. The caller must apply white
/// balance coefficients to the camera RGB before applying this matrix, since
/// the neutral axis is pinned on that assumption.
///
/// Returns None when the matrix is unusable, so the caller can fall back rather
/// than emit garbage.
pub fn cam_to_prophoto(xyz2cam: Mat3) -> Option<Mat3> {
    let prophoto2cam = normalize_rows(multiply(&xyz2cam, &PROPHOTO_TO_XYZ_D50))?;
    invert(&prophoto2cam)
}

pub fn prophoto_to_srgb() -> Mat3 {
    let adapt = bradford_adaptation(WHITE_D50, WHITE_D65);
    multiply(&XYZ_D65_TO_SRGB, &multiply(&adapt, &PROPHOTO_TO_XYZ_D50))
}

pub fn prophoto_to_srgb_cached() -> &'static Mat3 {
    static CACHED: OnceLock<Mat3> = OnceLock::new();
    CACHED.get_or_init(prophoto_to_srgb)
}

pub fn srgb_to_prophoto_cached() -> &'static Mat3 {
    static CACHED: OnceLock<Mat3> = OnceLock::new();
    CACHED.get_or_init(|| invert(&prophoto_to_srgb()).expect("sRGB primaries are independent"))
}

/// Lifts linear sRGB into the ProPhoto working space.
///
/// Used wherever data reaches the pipeline already resolved to sRGB, so that
/// exactly one working space exists downstream.
pub fn srgb_to_prophoto(c: [f32; 3]) -> [f32; 3] {
    apply(srgb_to_prophoto_cached(), c)
}

/// Converts linear ProPhoto to linear sRGB, desaturating a colour outside the
/// destination gamut toward its own luminance rather than clamping per channel,
/// which would shift both its hue and its luminance.
///
/// Mirrors gamut_clip_srgb in shaders/shader.wgsl; the two must stay in step.
pub fn prophoto_to_srgb_gamut_clipped(c: [f32; 3]) -> [f32; 3] {
    let srgb = apply(prophoto_to_srgb_cached(), c);
    let min_c = srgb[0].min(srgb[1]).min(srgb[2]);
    if min_c >= 0.0 {
        return srgb;
    }
    let luma = srgb[0] * SRGB_LUMA[0] + srgb[1] * SRGB_LUMA[1] + srgb[2] * SRGB_LUMA[2];
    if luma <= 0.0 {
        return [0.0; 3];
    }
    let t = luma / (luma - min_c);
    // Rounding can leave the darkest channel fractionally below zero, which
    // becomes NaN in the powf of a downstream display curve.
    srgb.map(|v| (luma + (v - luma) * t).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROMM_PRIMARIES: [[f64; 2]; 3] = [[0.7347, 0.2653], [0.1596, 0.8404], [0.0366, 0.0001]];
    const SRGB_PRIMARIES: [[f64; 2]; 3] = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]];

    /// Published Bradford D50 to D65 adaptation matrix.
    const BRADFORD_D50_TO_D65: [[f64; 3]; 3] = [
        [0.9555766, -0.0230393, 0.0631636],
        [-0.0282895, 1.0099416, 0.0210077],
        [0.0122982, -0.0204830, 1.3299098],
    ];

    fn invert_f64(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        let mut out = [[0.0f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                let (r0, r1) = ((j + 1) % 3, (j + 2) % 3);
                let (c0, c1) = ((i + 1) % 3, (i + 2) % 3);
                out[i][j] = (m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]) / det;
            }
        }
        out
    }

    /// Derives an RGB-to-XYZ matrix from primary chromaticities and a white
    /// point, independently of the constants under test.
    fn derive_rgb_to_xyz(primaries: [[f64; 2]; 3], white: [f64; 3]) -> [[f64; 3]; 3] {
        let mut m = [[0.0f64; 3]; 3];
        for (col, [x, y]) in primaries.iter().enumerate() {
            m[0][col] = x / y;
            m[1][col] = 1.0;
            m[2][col] = (1.0 - x - y) / y;
        }
        let inv = invert_f64(m);
        let scale: Vec<f64> = (0..3)
            .map(|r| (0..3).map(|c| inv[r][c] * white[c]).sum())
            .collect();

        let mut out = [[0.0f64; 3]; 3];
        for r in 0..3 {
            for c in 0..3 {
                out[r][c] = m[r][c] * scale[c];
            }
        }
        out
    }

    fn assert_matrix_close(actual: Mat3, expected: [[f64; 3]; 3], tol: f64, what: &str) {
        for r in 0..3 {
            for c in 0..3 {
                let diff = (actual[r][c] as f64 - expected[r][c]).abs();
                assert!(
                    diff < tol,
                    "{what}: [{r}][{c}] was {}, expected {} (diff {diff:.3e}, tol {tol:.1e})",
                    actual[r][c],
                    expected[r][c]
                );
            }
        }
    }

    fn assert_close(a: [f32; 3], b: [f32; 3], tol: f32, what: &str) {
        for i in 0..3 {
            assert!(
                (a[i] - b[i]).abs() < tol,
                "{what}: component {i} was {}, expected {} (tol {tol})",
                a[i],
                b[i]
            );
        }
    }

    #[test]
    fn prophoto_matrix_matches_primaries() {
        let derived = derive_rgb_to_xyz(ROMM_PRIMARIES, [0.96422, 1.0, 0.82521]);
        assert_matrix_close(PROPHOTO_TO_XYZ_D50, derived, 1e-6, "PROPHOTO_TO_XYZ_D50");
    }

    #[test]
    fn srgb_matrix_matches_primaries() {
        let derived = invert_f64(derive_rgb_to_xyz(SRGB_PRIMARIES, [0.95047, 1.0, 1.08883]));
        assert_matrix_close(XYZ_D65_TO_SRGB, derived, 1e-5, "XYZ_D65_TO_SRGB");
    }

    #[test]
    fn bradford_adaptation_matches_published() {
        let m = bradford_adaptation(WHITE_D50, WHITE_D65);
        assert_matrix_close(m, BRADFORD_D50_TO_D65, 1e-5, "Bradford D50->D65");
    }

    #[test]
    fn prophoto_white_is_d50() {
        assert_close(
            apply(&PROPHOTO_TO_XYZ_D50, [1.0; 3]),
            WHITE_D50,
            1e-4,
            "ProPhoto white",
        );
    }

    #[test]
    fn prophoto_white_maps_to_srgb_white() {
        assert_close(
            apply(&prophoto_to_srgb(), [1.0; 3]),
            [1.0; 3],
            2e-3,
            "white",
        );
    }

    #[test]
    fn invert_round_trips() {
        let inv = invert(&PROPHOTO_TO_XYZ_D50).unwrap();
        let identity = multiply(&PROPHOTO_TO_XYZ_D50, &inv);
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((identity[i][j] - expected).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn invert_rejects_ill_conditioned() {
        let near_singular: Mat3 = [
            [0.5, 0.3, 0.2],
            [0.25, 0.15, 0.60],
            [0.5000001, 0.3, 0.1999999],
        ];
        assert!(
            invert(&near_singular).is_none(),
            "guard let a near-singular matrix through"
        );
    }

    #[test]
    fn prophoto_luma_is_the_y_row() {
        let derived = derive_rgb_to_xyz(ROMM_PRIMARIES, [0.96422, 1.0, 0.82521]);
        for c in 0..3 {
            let diff = (PROPHOTO_LUMA[c] as f64 - derived[1][c]).abs();
            assert!(
                diff < 1e-6,
                "PROPHOTO_LUMA[{c}] was {} (diff {diff:.3e})",
                PROPHOTO_LUMA[c]
            );
        }
    }

    #[test]
    fn cam_to_prophoto_is_identity_when_camera_is_prophoto() {
        let xyz2cam = invert(&PROPHOTO_TO_XYZ_D50).unwrap();
        let m = cam_to_prophoto(xyz2cam).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (m[i][j] - expected).abs() < 1e-4,
                    "[{i}][{j}] was {}, expected {expected}",
                    m[i][j]
                );
            }
        }
    }

    #[test]
    fn cam_to_prophoto_pins_neutral_to_white() {
        let xyz2cam: Mat3 = [
            [0.7034, -0.1662, -0.0499],
            [-0.5607, 1.3411, 0.2450],
            [-0.1163, 0.2355, 0.6446],
        ];
        let m = cam_to_prophoto(xyz2cam).unwrap();
        assert_close(apply(&m, [1.0; 3]), [1.0; 3], 1e-4, "camera neutral");
    }

    /// The shader carries these rows as literals; drift between the two would
    /// silently change every rendered image.
    /// Chroma read as colour minus luminance shrinks when the working space
    /// widens, because a real colour sits nearer the neutral axis. Any filter
    /// gating on a fixed chroma difference has to be rescaled by the same
    /// amount or it reaches further than it was set to.
    #[test]
    fn chroma_differences_shrink_by_the_factor_the_shader_divides_by() {
        const SHADER_CHROMA_TOL: (f32, f32) = (0.143, 0.057);
        const ORIGINAL_CHROMA_TOL: (f32, f32) = (0.20, 0.08);
        const PROPHOTO_LUMA_W: [f32; 3] = [0.2880402, 0.7118741, 0.0000857];
        const SRGB_LUMA_W: [f32; 3] = [0.2126, 0.7152, 0.0722];

        fn chroma(c: [f32; 3], w: [f32; 3]) -> f32 {
            let y = c[0] * w[0] + c[1] * w[1] + c[2] * w[2];
            ((c[0] - y).powi(2) + (c[2] - y).powi(2)).sqrt()
        }

        let srgb_to_prophoto = invert(&prophoto_to_srgb()).unwrap();
        let mut ratios = Vec::new();
        for i in 0..720 {
            let angle = i as f32 * std::f32::consts::TAU / 720.0;
            for axis in 0..3 {
                let mut direction = [0.0f32; 3];
                direction[axis] = angle.cos();
                direction[(axis + 1) % 3] = angle.sin();
                let wide = chroma(apply(&srgb_to_prophoto, direction), PROPHOTO_LUMA_W);
                if wide > 1e-6 {
                    ratios.push(chroma(direction, SRGB_LUMA_W) / wide);
                }
            }
        }
        ratios.sort_by(f32::total_cmp);
        let median = ratios[ratios.len() / 2];

        assert!(
            (median - 1.40).abs() < 0.05,
            "chroma shrank by {median}, not the 1.40 the tolerances assume"
        );
        for (scaled, original) in [
            (SHADER_CHROMA_TOL.0, ORIGINAL_CHROMA_TOL.0),
            (SHADER_CHROMA_TOL.1, ORIGINAL_CHROMA_TOL.1),
        ] {
            let expected = original / median;
            assert!(
                (scaled - expected).abs() < 0.005,
                "tolerance {scaled} should be {expected:.3}, which is {original} over {median:.2}"
            );
        }
    }

    /// Luminance is the same quantity in both spaces, since each normalises
    /// white to one, so the tolerances measured against it were left alone.
    #[test]
    fn luminance_differences_do_not_shrink() {
        const PROPHOTO_LUMA_W: [f32; 3] = [0.2880402, 0.7118741, 0.0000857];
        const SRGB_LUMA_W: [f32; 3] = [0.2126, 0.7152, 0.0722];
        fn luma(c: [f32; 3], w: [f32; 3]) -> f32 {
            c[0] * w[0] + c[1] * w[1] + c[2] * w[2]
        }

        let srgb_to_prophoto = invert(&prophoto_to_srgb()).unwrap();
        let mut ratios = Vec::new();
        for i in 0..720 {
            let angle = i as f32 * std::f32::consts::TAU / 720.0;
            for axis in 0..3 {
                let mut direction = [0.0f32; 3];
                direction[axis] = angle.cos();
                direction[(axis + 1) % 3] = angle.sin();
                let wide = luma(apply(&srgb_to_prophoto, direction), PROPHOTO_LUMA_W).abs();
                let narrow = luma(direction, SRGB_LUMA_W).abs();
                if wide > 1e-4 && narrow > 1e-4 {
                    ratios.push(narrow / wide);
                }
            }
        }
        ratios.sort_by(f32::total_cmp);
        let median = ratios[ratios.len() / 2];
        assert!(
            (median - 1.0).abs() < 0.02,
            "luminance scaled by {median}, so its tolerances would need changing too"
        );
    }

    /// Clarity raises the ratio between a pixel and its surroundings to the
    /// slider's power. A bright specularity on near black drives that ratio
    /// into the hundreds, which is a division artefact rather than detail.
    #[test]
    fn clarity_gain_is_bounded_without_touching_real_detail() {
        const SHADER_DETAIL_STOPS: f32 = 2.0;
        let gain = |centre: f32, blurred: f32, amount: f32| {
            let log_ratio = (centre / blurred)
                .log2()
                .clamp(-SHADER_DETAIL_STOPS, SHADER_DETAIL_STOPS);
            (log_ratio * amount).exp2()
        };

        // Ordinary texture and a strong edge have to come through untouched.
        assert!((gain(0.20, 0.18, 0.8) - (0.20f32 / 0.18).powf(0.8)).abs() < 1e-4);
        assert!((gain(0.40, 0.15, 0.8) - (0.40f32 / 0.15).powf(0.8)).abs() < 1e-4);

        // The pathological cases are what the bound is for.
        assert!(
            gain(0.50, 0.010, 0.8) < 4.0,
            "specular on dark reached {}",
            gain(0.50, 0.010, 0.8)
        );
        assert!(
            gain(0.50, 0.001, 0.8) < 4.0,
            "specular on black reached {}",
            gain(0.50, 0.001, 0.8)
        );
    }

    #[test]
    fn shader_constants_match_this_module() {
        const SHADER_PROPHOTO_TO_SRGB: Mat3 = [
            [2.0340760, -0.7273341, -0.3067416],
            [-0.2288132, 1.2317301, -0.0029170],
            [-0.0085698, -0.1532867, 1.1618567],
        ];
        const SHADER_SRGB_TO_PROPHOTO: Mat3 = [
            [0.5293459, 0.3300728, 0.1405812],
            [0.0983743, 0.8734611, 0.0281647],
            [0.0168832, 0.1176725, 0.8654441],
        ];
        const SHADER_LUMA: [f32; 3] = [0.2880402, 0.7118741, 0.0000857];

        let pp2s = prophoto_to_srgb();
        for r in 0..3 {
            for c in 0..3 {
                assert!(
                    (pp2s[r][c] - SHADER_PROPHOTO_TO_SRGB[r][c]).abs() < 1e-6,
                    "PROPHOTO_TO_SRGB[{r}][{c}]: module {} vs shader {}",
                    pp2s[r][c],
                    SHADER_PROPHOTO_TO_SRGB[r][c]
                );
            }
        }
        let s2pp = invert(&pp2s).unwrap();
        for r in 0..3 {
            for c in 0..3 {
                assert!(
                    (s2pp[r][c] - SHADER_SRGB_TO_PROPHOTO[r][c]).abs() < 1e-6,
                    "SRGB_TO_PROPHOTO[{r}][{c}]: module {} vs shader {}",
                    s2pp[r][c],
                    SHADER_SRGB_TO_PROPHOTO[r][c]
                );
            }
        }
        for c in 0..3 {
            assert!((PROPHOTO_LUMA[c] - SHADER_LUMA[c]).abs() < 1e-7);
        }
    }

    #[test]
    fn srgb_to_prophoto_round_trips() {
        for c in [[0.2, 0.5, 0.9], [1.0, 0.0, 0.0], [0.05, 0.05, 0.05]] {
            let back = prophoto_to_srgb_gamut_clipped(srgb_to_prophoto(c));
            assert_close(back, c, 1e-4, "sRGB round trip");
        }
    }

    #[test]
    fn gamut_clip_never_returns_negative() {
        for c in [
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
        ] {
            let out = prophoto_to_srgb_gamut_clipped(c);
            assert!(
                out.iter().all(|v| *v >= 0.0),
                "negative channel survived for {c:?}: {out:?}"
            );
        }
    }

    #[test]
    fn gamut_clip_preserves_in_gamut_colours() {
        let neutral = prophoto_to_srgb_gamut_clipped([0.5, 0.5, 0.5]);
        let direct = apply(&prophoto_to_srgb(), [0.5, 0.5, 0.5]);
        assert_close(neutral, direct, 1e-6, "in-gamut colour was altered");
    }

    #[test]
    fn gamut_clip_lifts_out_of_gamut_to_the_boundary() {
        // A saturated ProPhoto green lands outside sRGB.
        let out = prophoto_to_srgb_gamut_clipped([0.0, 1.0, 0.0]);
        let min_c = out[0].min(out[1]).min(out[2]);
        assert!(min_c >= -1e-6, "still outside gamut: {min_c}");

        let before = apply(&prophoto_to_srgb(), [0.0, 1.0, 0.0]);
        let luma_before = before[0] * 0.2126 + before[1] * 0.7152 + before[2] * 0.0722;
        let luma_after = out[0] * 0.2126 + out[1] * 0.7152 + out[2] * 0.0722;
        assert!(
            (luma_before - luma_after).abs() < 1e-4,
            "luminance moved: {luma_before} -> {luma_after}"
        );
    }

    #[test]
    fn cam_to_prophoto_rejects_degenerate_matrix() {
        assert!(cam_to_prophoto([[0.0; 3]; 3]).is_none());
    }
}
