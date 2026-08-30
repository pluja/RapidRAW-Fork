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

/// Grain at the top of the slider, past any emulsion.
pub const MAX_AMPLITUDE: f32 = 0.10;

/// Grain amplitude for a slider position from zero to one hundred.
pub fn amplitude(slider: f32) -> f32 {
    (slider / 100.0).clamp(0.0, 1.0) * MAX_AMPLITUDE
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Weight for an octave whose cells span this many pixels.
///
/// A pixel smaller than a cell sees one cell and keeps the octave whole. A pixel
/// larger than a cell averages about `1 / cell_pixels` independent cells, and
/// averaging n independent samples divides their spread by sqrt(n), so what
/// survives goes as the square root of the cell size. That is the same reason
/// grain softens when a print is made smaller.
///
/// This used to fade out with `smoothstep(1.0, 2.5, cell_pixels)`, which is a
/// cliff rather than a falloff: it zeroed every octave under a pixel and did not
/// reach full weight until two and a half. At the default size a preview showed
/// eight percent of the grain the export received, and the whole lower half of
/// the size slider weighed nothing at all in a preview.
pub const OCTAVE_FALLOFF: f32 = 0.9;

pub fn octave_weight(cell_pixels: f32) -> f32 {
    cell_pixels.clamp(0.0, 1.0).powf(OCTAVE_FALLOFF)
}

/// Cells per pixel. Grain keeps its size relative to the picture at every
/// render resolution, which is what makes a preview agree with an export.
pub fn frequency(grain_size: f32, scale: f32) -> f32 {
    (1.0 / grain_size.max(0.1)) / scale
}

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
pub fn grain_noise(x: f32, y: f32, roughness: f32, cell_pixels: f32) -> f32 {
    let fine = value_noise(x * 2.0 + 7.1, y * 2.0 + 31.7);
    let mid = value_noise(x, y);
    let coarse = value_noise(x * 0.5 + 19.7, y * 0.5 + 4.3);

    let base_fine = 0.55 + (0.20 - 0.55) * roughness;
    let base_coarse = 0.25 + (0.85 - 0.25) * roughness;
    let total = (base_fine * base_fine + 1.0 + base_coarse * base_coarse).sqrt();

    let w_fine = base_fine * octave_weight(cell_pixels * 0.5);
    let w_mid = octave_weight(cell_pixels);
    let w_coarse = base_coarse * octave_weight(cell_pixels * 2.0);
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
    use crate::shader_probe;

    /// The property the model exists for: what you see while editing has to be
    /// what lands in the file. A preview is a smaller print of the same frame,
    /// so its grain should match the exported grain box-downsampled to preview
    /// size. Measured on real fields rather than argued from the weights.
    fn field_sd(width: usize, height: usize, freq: f32, cell_pixels: f32, roughness: f32) -> f32 {
        let mut sum = 0.0f64;
        let mut sum_sq = 0.0f64;
        for y in 0..height {
            for x in 0..width {
                let v = grain_noise(x as f32 * freq, y as f32 * freq, roughness, cell_pixels) as f64;
                sum += v;
                sum_sq += v * v;
            }
        }
        let n = (width * height) as f64;
        ((sum_sq / n) - (sum / n) * (sum / n)).max(0.0).sqrt() as f32
    }

    fn downsampled_sd(width: usize, height: usize, freq: f32, cell_pixels: f32, roughness: f32, factor: usize) -> f32 {
        let (dw, dh) = (width / factor, height / factor);
        let mut vals = Vec::with_capacity(dw * dh);
        for by in 0..dh {
            for bx in 0..dw {
                let mut acc = 0.0f64;
                for y in 0..factor {
                    for x in 0..factor {
                        let (px, py) = (bx * factor + x, by * factor + y);
                        acc += grain_noise(px as f32 * freq, py as f32 * freq, roughness, cell_pixels) as f64;
                    }
                }
                vals.push(acc / (factor * factor) as f64);
            }
        }
        let n = vals.len() as f64;
        let mean = vals.iter().sum::<f64>() / n;
        (vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n).sqrt() as f32
    }

    /// The shader is the live path. A constant checked against another Rust
    /// literal proves nothing about what runs on the GPU, so this reads the
    /// shader source.
    #[test]
    fn the_shader_fades_octaves_the_way_this_module_measured() {
        let shader = shader_probe::f32_const("GRAIN_OCTAVE_FALLOFF");
        assert!(
            (shader - OCTAVE_FALLOFF).abs() < 1e-9,
            "shader fades octaves at {shader}, this module measured {OCTAVE_FALLOFF}"
        );

        let body = shader_probe::fn_body("grain_octave_weight");
        assert!(
            body.contains("GRAIN_OCTAVE_FALLOFF") && !body.contains("smoothstep"),
            "grain_octave_weight is not the measured falloff: {body:?}"
        );
    }

    #[test]
    fn the_shader_grains_at_the_amplitude_this_module_reasons_about() {
        for (mirrored, name) in [
            (MAX_AMPLITUDE, "GRAIN_MAX_AMPLITUDE"),
            (LAYER_INDEPENDENCE, "GRAIN_LAYER_INDEPENDENCE"),
        ] {
            let shader = shader_probe::f32_const(name);
            assert!(
                (mirrored - shader).abs() < 1e-9,
                "{name} is {shader} in the shader, {mirrored} here"
            );
        }
    }

    #[test]
    fn preview_grain_matches_the_export_it_previews() {
        const FACTOR: usize = 4;
        let (ew, eh) = (256usize, 256usize);
        let export_scale = 3.85f32;

        for &slider in &[15.0f32, 25.0, 35.0, 50.0] {
            for &roughness in &[0.3f32, 0.7] {
                let grain_size = slider / 50.0;

                let export_freq = frequency(grain_size, export_scale);
                let export_cell = grain_size * export_scale;
                let shrunk = downsampled_sd(ew, eh, export_freq, export_cell, roughness, FACTOR);

                let preview_scale = export_scale / FACTOR as f32;
                let preview_freq = frequency(grain_size, preview_scale);
                let preview_cell = grain_size * preview_scale;
                let preview = field_sd(ew / FACTOR, eh / FACTOR, preview_freq, preview_cell, roughness);

                let ratio = preview / shrunk.max(1e-6);
                assert!(
                    (0.90..1.25).contains(&ratio),
                    "size {slider}, roughness {roughness}: preview grain is {ratio:.3}x the \
                     export downsampled to the same size (preview sd {preview:.4}, \
                     shrunk export sd {shrunk:.4})"
                );
            }
        }
    }

    #[test]
    fn every_size_on_the_slider_produces_grain_in_a_preview() {
        // At a 1080-class preview the whole lower half of the Size slider used to
        // weigh exactly zero, so grain was invisible while editing and present in
        // the file.
        for slider in 1..=50 {
            let grain_size = slider as f32 / 50.0;
            let scale = 1.0f32;
            let cell = grain_size * scale;
            let sd = field_sd(64, 64, frequency(grain_size, scale), cell, 0.7);
            assert!(
                sd > 0.01,
                "size {slider} renders grain with standard deviation {sd} in a preview"
            );
        }
    }


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
                    (0..90)
                        .map(move |j| grain_noise(i as f32 * 0.37, j as f32 * 0.53, roughness, 6.0))
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

    /// Real emulsions span under three to one from the finest colour stock to
    /// the coarsest, so the slider should spend its middle on that range rather
    /// than racing past it in the first quarter.
    #[test]
    fn the_amount_slider_spends_its_middle_on_film() {
        assert!(amplitude(0.0) < 1e-6, "zero should mean no grain");
        assert!(
            (amplitude(100.0) - MAX_AMPLITUDE).abs() < 1e-6,
            "the top should reach the stated maximum"
        );

        // The band where real stocks live carries a film-like spread.
        let film_range = amplitude(70.0) / amplitude(40.0);
        assert!(
            (1.7..2.6).contains(&film_range),
            "forty to seventy spanned {film_range}x, film spans under three"
        );

        // Past the coarsest stock there is still somewhere to go.
        assert!(
            amplitude(100.0) > amplitude(70.0) * 1.4,
            "the creative end gave nothing beyond film"
        );

        // Monotonic, or the slider would fold back on itself.
        let mut previous = -1.0f32;
        for step in 0..=100 {
            let current = amplitude(step as f32);
            assert!(current > previous, "amplitude fell at {step}");
            previous = current;
        }
    }

    /// Grain belongs to the picture, not to the buffer it is drawn into, so a
    /// cell has to cover the same fraction of the frame at every render size.
    /// Holding a minimum pixel size instead made preview grain several times
    /// coarser than the export, which read as blocky until the view was zoomed.
    #[test]
    fn grain_keeps_its_size_relative_to_the_picture() {
        for size in [10.0f32, 20.0, 50.0, 100.0] {
            let mut fractions = Vec::new();
            for render_px in [1200.0f32, 1600.0, 2400.0, 6240.0] {
                let scale = render_px / 1080.0;
                let cell_px = 1.0 / frequency(size / 50.0, scale);
                fractions.push(cell_px / render_px);
            }
            let first = fractions[0];
            for f in &fractions {
                assert!(
                    (f - first).abs() / first < 1e-4,
                    "size {size} covered {f} of the frame against {first} elsewhere"
                );
            }
        }
    }

    /// Grain finer than the pixels drawing it averages away, as it does when a
    /// print is made smaller, so a preview should show less of it rather than a
    /// coarser version of it.
    #[test]
    fn grain_quietens_rather_than_coarsens_when_it_cannot_be_resolved() {
        let sample = |cell: f32| {
            let values: Vec<f32> = (0..80)
                .flat_map(|i| {
                    (0..80).map(move |j| grain_noise(i as f32 * 0.37, j as f32 * 0.53, 0.5, cell))
                })
                .collect();
            stdev(&values)
        };

        let resolved = sample(8.0);
        let marginal = sample(1.6);
        let unresolvable = sample(0.4);

        assert!(marginal < resolved, "grain did not quieten as cells shrank");
        assert!(
            unresolvable < marginal * 0.6,
            "sub-pixel grain still carried {unresolvable} against {marginal}"
        );
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
                    grain_noise(x, y, 0.5, 6.0),
                    grain_noise(x + 53.7, y + 11.3, 0.5, 6.0),
                    grain_noise(x + 97.1, y + 61.9, 0.5, 6.0),
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
