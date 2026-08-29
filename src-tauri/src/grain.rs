//! Reference for the film grain in shaders/shader.wgsl.
//!
//! Grain comes from silver halide crystals scattered through the emulsion as a
//! Poisson process. A crystal either develops or it does not, so the count under
//! a pixel is binomial and its spread follows the square root of density times
//! its complement: none where the emulsion is clear, none where it is fully
//! developed, most in between.
//!
//! The shader carries its own copy for the GPU. This one exists so the
//! properties that make the texture read as grain can be measured.

#![allow(dead_code)]

/// How much each dye layer's grain differs from the others.
pub const LAYER_INDEPENDENCE: f32 = 0.30;

fn hash(x: f32, y: f32) -> f32 {
    let mut p3 = [
        (x * 0.1031).fract(),
        (y * 0.1031).fract(),
        (x * 0.1031).fract(),
    ];
    let d = p3[0] * (p3[1] + 33.33) + p3[1] * (p3[2] + 33.33) + p3[2] * (p3[0] + 33.33);
    for v in p3.iter_mut() {
        *v += d;
    }
    ((p3[0] + p3[1]) * p3[2]).fract()
}

/// One random value per cell, smoothly interpolated.
///
/// The gradient noise this replaced evaluated to exactly zero at every integer
/// coordinate, leaving a regular grid of grain-free pixels across the frame.
pub fn value_noise(x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    let a = hash(ix, iy);
    let b = hash(ix + 1.0, iy);
    let c = hash(ix, iy + 1.0);
    let d = hash(ix + 1.0, iy + 1.0);

    let lower = a + (b - a) * ux;
    let upper = c + (d - c) * ux;
    (lower + (upper - lower) * uy) * 2.0 - 1.0
}

/// Octaves summed to a broad band, with roughness moving weight toward the
/// coarser ones rather than trading the finer ones away.
pub fn grain_noise(x: f32, y: f32, roughness: f32) -> f32 {
    let fine = value_noise(x * 2.0 + 7.1, y * 2.0 + 31.7);
    let mid = value_noise(x, y);
    let coarse = value_noise(x * 0.5 + 19.7, y * 0.5 + 4.3);

    let w_fine = 0.55 + (0.20 - 0.55) * roughness;
    let w_mid = 1.0f32;
    let w_coarse = 0.25 + (0.85 - 0.25) * roughness;

    let total = (w_fine * w_fine + w_mid * w_mid + w_coarse * w_coarse).sqrt();
    (fine * w_fine + mid * w_mid + coarse * w_coarse) / total
}

/// Spread of the grain at a given density, from the binomial count of crystals.
pub fn spread(density: f32) -> f32 {
    let d = density.clamp(0.0, 1.0);
    (d * (1.0 - d)).max(0.0).sqrt() * 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stdev(values: &[f32]) -> f32 {
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / values.len() as f32).sqrt()
    }

    /// The gradient noise this replaced returned exactly zero at every integer
    /// coordinate, so a grid of pixels across the frame carried no grain at all.
    #[test]
    fn the_noise_has_no_dead_lattice() {
        let mut zeros = 0;
        for i in 0..16 {
            for j in 0..16 {
                if value_noise(i as f32, j as f32).abs() < 1e-9 {
                    zeros += 1;
                }
            }
        }
        assert_eq!(zeros, 0, "{zeros} of 256 lattice points carried no grain");
    }

    /// Roughness should change the texture, not how much of it there is. The
    /// crossfade it replaced dropped an octave entirely at the top of the range.
    #[test]
    fn roughness_changes_character_not_amount() {
        let sample = |roughness: f32| {
            let values: Vec<f32> = (0..90)
                .flat_map(|i| {
                    (0..90).map(move |j| grain_noise(i as f32 * 0.37, j as f32 * 0.53, roughness))
                })
                .collect();
            stdev(&values)
        };

        let quiet = sample(0.0);
        let rough = sample(1.0);
        assert!(
            (quiet - rough).abs() / quiet < 0.05,
            "amount moved from {quiet} to {rough} across the roughness range"
        );
    }

    /// A crystal either develops or it does not, so there is nothing to vary
    /// where the emulsion is clear or fully developed, and most to vary between.
    #[test]
    fn spread_follows_the_binomial_count() {
        assert!(spread(0.0) < 1e-6, "clear emulsion carried grain");
        assert!(spread(1.0) < 1e-6, "fully developed emulsion carried grain");
        assert!(
            (spread(0.5) - 1.0).abs() < 1e-6,
            "mid grey should be the peak"
        );

        // Symmetric about mid grey, and rising toward it from either side.
        for step in 1..50 {
            let d = step as f32 / 100.0;
            assert!(
                (spread(d) - spread(1.0 - d)).abs() < 1e-5,
                "spread was not symmetric at {d}"
            );
            assert!(
                spread(d) < spread(d + 0.01),
                "spread fell approaching mid grey"
            );
        }
    }

    /// Three dye layers each carry their own grain, but sharing most of it is
    /// what keeps the result reading as grain rather than as colour noise.
    #[test]
    fn layers_differ_without_becoming_colour_noise() {
        let mut differences = Vec::new();
        for i in 0..40 {
            for j in 0..40 {
                let (x, y) = (i as f32 * 0.41, j as f32 * 0.59);
                let layers = [
                    grain_noise(x, y, 0.5),
                    grain_noise(x + 53.7, y + 11.3, 0.5),
                    grain_noise(x + 97.1, y + 61.9, 0.5),
                ];
                let mean = layers.iter().sum::<f32>() / 3.0;
                let mixed: Vec<f32> = layers
                    .iter()
                    .map(|l| mean + (l - mean) * LAYER_INDEPENDENCE)
                    .collect();
                differences.push(mixed[0] - mixed[1]);
            }
        }
        let spread_between_layers = stdev(&differences);
        assert!(
            spread_between_layers > 0.01,
            "the layers were identical, which is black and white grain"
        );
        assert!(
            spread_between_layers < 0.30,
            "the layers diverged by {spread_between_layers}, which reads as colour noise"
        );
    }
}
