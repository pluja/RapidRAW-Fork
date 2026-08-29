//! Linear ProPhoto RGB (ROMM, D50) working space.
//!
//! Chosen over sRGB because a camera sensor records well outside the sRGB gamut,
//! and over Rec.2020 because the DNG spec defines DCP ProfileHueSatMap and
//! ProfileLookTable against ProPhoto primaries; keeping the pipeline there lets
//! camera profiles apply without a round trip.

pub type Mat3 = [[f32; 3]; 3];

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

pub fn invert(m: &Mat3) -> Option<Mat3> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-12 {
        return None;
    }
    let inv_det = 1.0 / det;

    let mut out = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let (r0, r1) = ((j + 1) % 3, (j + 2) % 3);
            let (c0, c1) = ((i + 1) % 3, (i + 2) % 3);
            out[i][j] = (m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]) * inv_det;
        }
    }
    Some(out)
}

/// Scales each row to sum to 1 so a white-balanced camera neutral maps to
/// (1, 1, 1) in the destination space, which is what makes an explicit
/// chromatic adaptation of the camera matrix unnecessary.
fn normalize_rows(m: Mat3) -> Mat3 {
    let mut out = m;
    for row in out.iter_mut() {
        let sum: f32 = row.iter().sum();
        if sum.abs() > f32::EPSILON {
            for c in row.iter_mut() {
                *c /= sum;
            }
        }
    }
    out
}

fn bradford_adaptation(src_white: [f32; 3], dst_white: [f32; 3]) -> Mat3 {
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
/// `xyz2cam` is the camera's DNG colour matrix. Returns None when the matrix is
/// singular, in which case the caller should fall back rather than emit garbage.
pub fn cam_to_prophoto(xyz2cam: Mat3) -> Option<Mat3> {
    let prophoto2cam = normalize_rows(multiply(&xyz2cam, &PROPHOTO_TO_XYZ_D50));
    invert(&prophoto2cam)
}

pub fn prophoto_to_srgb() -> Mat3 {
    let adapt = bradford_adaptation(WHITE_D50, WHITE_D65);
    multiply(&XYZ_D65_TO_SRGB, &multiply(&adapt, &PROPHOTO_TO_XYZ_D50))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn prophoto_white_is_d50() {
        let white = apply(&PROPHOTO_TO_XYZ_D50, [1.0, 1.0, 1.0]);
        assert_close(white, WHITE_D50, 1e-4, "ProPhoto (1,1,1)");
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
    fn prophoto_white_maps_to_srgb_white() {
        let m = prophoto_to_srgb();
        assert_close(apply(&m, [1.0, 1.0, 1.0]), [1.0, 1.0, 1.0], 2e-3, "white");
    }

    #[test]
    fn luma_weights_sum_to_one() {
        let sum: f32 = PROPHOTO_LUMA.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "luma weights summed to {sum}");
    }
}
