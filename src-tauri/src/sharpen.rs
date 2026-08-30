//! Reference for the sharpening mask in shaders/shader.wgsl.
//!
//! Sharpening itself works on the pixel grid, so how much of it an export
//! receives depends on the export's resolution, the way it does in Lightroom.
//! The mask is different: it is drawn on screen as the thing the Masking slider
//! is set by, so it has to describe the same picture whatever size that picture
//! is currently rendered at.
//!
//! The shader carries its own copy for the GPU. This one exists so the
//! agreement between a preview's mask and an export's can be measured.

#![allow(dead_code)]

/// Edge strength either side of which sharpening is held back, at full masking.
///
/// These were chosen by eye against a gradient measured in pixels, which means
/// they are calibrated at the render size where `scale` is one: the shader's
/// REFERENCE_DIMENSION of 1080 on the short side. A frame that size renders
/// exactly as it did before the gradient was normalised; larger ones now agree
/// with it instead of protecting more the bigger they get.
pub const MASK_KNEE_LOW: f32 = 0.02;
pub const MASK_KNEE_HIGH: f32 = 0.34;

/// Short side at which `scale` is one, and so the size the knees above mean
/// what they were set to mean.
pub const REFERENCE_SHORT_SIDE: f32 = 1080.0;

/// Weights across the five-tap window, which taper so that the gradient does
/// not crawl with sensor noise the way a three by three would.
const TAPER: [f32; 5] = [1.0, 2.0, 3.0, 2.0, 1.0];

/// Sum of |offset| * taper over the window, which is what `edge` divides by.
const GRADIENT_NORMALISER: f32 = 9.0;

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Edge strength at a pixel, expressed per fraction of the frame rather than
/// per pixel.
///
/// `scale` is the render's short side over 1080, the same figure the shader
/// computes. Without it the same edge reads weaker the more pixels it is spread
/// across, so an export measures a shallower slope than the preview it was set
/// from and the mask protects areas the photographer chose to sharpen.
pub fn edge_strength(sample: impl Fn(i32, i32) -> f32, scale: f32) -> f32 {
    let mut gx = 0.0f32;
    let mut gy = 0.0f32;

    for iy in 0..5i32 {
        for ix in 0..5i32 {
            let (ox, oy) = (ix - 2, iy - 2);
            let value = sample(ox, oy);
            gx += value * ox as f32 * TAPER[iy as usize];
            gy += value * oy as f32 * TAPER[ix as usize];
        }
    }

    (gx * gx + gy * gy).sqrt() / GRADIENT_NORMALISER * scale
}

/// How much of the sharpening a pixel of this edge strength receives.
pub fn mask(edge: f32, masking: f32) -> f32 {
    if masking <= 0.001 {
        return 1.0;
    }
    let knee = MASK_KNEE_LOW + (MASK_KNEE_HIGH - MASK_KNEE_LOW) * masking;
    1.0 + (smoothstep(knee * 0.15, knee, edge) - 1.0) * masking
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_probe;

    /// A soft edge running down the frame, as a fraction of the frame's width.
    ///
    /// Rendering it at two resolutions is the whole point: it is the same
    /// picture, so the mask has to read the same on both.
    fn ramp(width_px: f32, edge_width_fraction: f32) -> impl Fn(f32) -> f32 {
        move |x_px: f32| {
            let position = x_px / width_px;
            smoothstep(
                0.5 - edge_width_fraction * 0.5,
                0.5 + edge_width_fraction * 0.5,
                position,
            )
        }
    }

    fn scale_of(short_side_px: f32) -> f32 {
        (short_side_px / REFERENCE_SHORT_SIDE).max(0.1)
    }

    fn edge_at_centre(width_px: f32, edge_width_fraction: f32) -> f32 {
        let field = ramp(width_px, edge_width_fraction);
        let centre = width_px * 0.5;
        edge_strength(|ox, _oy| field(centre + ox as f32), scale_of(width_px))
    }

    /// The property the module exists for. A preview and an export are the same
    /// picture at two sizes, so the Masking slider has to protect the same
    /// parts of it in both.
    #[test]
    fn the_mask_reads_the_same_at_preview_and_export_scale() {
        let preview_px = 2048.0;
        let export_px = 6000.0;

        for &edge_width in &[0.01f32, 0.02, 0.05, 0.10] {
            for &masking in &[0.25f32, 0.5, 0.75, 1.0] {
                let preview = mask(edge_at_centre(preview_px, edge_width), masking);
                let export = mask(edge_at_centre(export_px, edge_width), masking);
                assert!(
                    (preview - export).abs() < 0.05,
                    "edge {edge_width}, masking {masking}: preview mask {preview:.3} \
                     against export mask {export:.3}"
                );
            }
        }
    }

    /// What the scaling is there to stop. Without it the export measured a
    /// third of the preview's slope and masked away sharpening the preview
    /// had shown being applied.
    #[test]
    fn an_unscaled_gradient_would_disagree_across_resolutions() {
        let edge_width = 0.02;
        let unscaled = |width_px: f32| {
            let field = ramp(width_px, edge_width);
            let centre = width_px * 0.5;
            edge_strength(|ox, _oy| field(centre + ox as f32), 1.0)
        };

        let preview = mask(unscaled(2048.0), 1.0);
        let export = mask(unscaled(6000.0), 1.0);
        assert!(
            preview - export > 0.3,
            "the unscaled gradient should disagree, but gave {preview:.3} and {export:.3}"
        );
    }

    /// Flat areas stay protected and real detail still gets through, or the
    /// scaling would have bought agreement by disabling the control.
    #[test]
    fn masking_still_separates_flat_from_detailed() {
        let flat = mask(edge_strength(|_, _| 0.5, scale_of(2048.0)), 1.0);
        assert!(flat < 0.05, "a flat field took {flat:.3} of the sharpening");

        let detailed = mask(edge_at_centre(2048.0, 0.004), 1.0);
        assert!(
            detailed > 0.9,
            "a hard edge took only {detailed:.3} of the sharpening"
        );
    }

    #[test]
    fn masking_off_leaves_every_pixel_sharpened() {
        for edge in [0.0f32, 0.01, 0.5, 10.0] {
            assert_eq!(mask(edge, 0.0), 1.0);
        }
    }

    #[test]
    fn the_shader_knees_match_this_module() {
        for (mirrored, name) in [
            (MASK_KNEE_LOW, "SHARPEN_MASK_KNEE_LOW"),
            (MASK_KNEE_HIGH, "SHARPEN_MASK_KNEE_HIGH"),
        ] {
            let shader = shader_probe::f32_const(name);
            assert!(
                (mirrored - shader).abs() < 1e-9,
                "{name} is {shader} in the shader, {mirrored} here"
            );
        }
    }

    /// The knees mean what they were set to mean only at the size where scale
    /// is one, so moving the shader's reference would silently recalibrate the
    /// Masking slider.
    #[test]
    fn the_reference_size_the_knees_are_anchored_at_is_the_shaders_own() {
        let shader = shader_probe::f32_const("REFERENCE_DIMENSION");
        assert!(
            (shader - REFERENCE_SHORT_SIDE).abs() < 1e-9,
            "the shader scales against {shader}, the knees were set against {REFERENCE_SHORT_SIDE}"
        );
    }

    /// A frame at the reference size has to render exactly as it did before the
    /// gradient was normalised, or the change was a recalibration rather than a
    /// fix.
    #[test]
    fn the_reference_size_is_unchanged_by_the_normalisation() {
        let field = ramp(REFERENCE_SHORT_SIDE, 0.02);
        let centre = REFERENCE_SHORT_SIDE * 0.5;
        let sample = |ox: i32, _oy: i32| field(centre + ox as f32);
        let normalised = edge_strength(sample, scale_of(REFERENCE_SHORT_SIDE));
        let unnormalised = edge_strength(sample, 1.0);
        assert!((normalised - unnormalised).abs() < 1e-6);
    }

    /// The constant is only worth pinning if the shader applies it, and it is
    /// the multiply rather than the value that makes the mask agree.
    #[test]
    fn the_shader_scales_the_edge_it_masks_on() {
        let body = shader_probe::fn_body("apply_sharpen");
        let edge_line = body
            .lines()
            .find(|l| l.trim_start().starts_with("let edge ="))
            .expect("apply_sharpen no longer computes an edge");
        assert!(
            edge_line.contains("* scale"),
            "apply_sharpen measures its mask in pixels again: {edge_line:?}"
        );
    }
}
