use crate::color_space::{self, Mat3};
use crate::image_processing::apply_orientation;
use anyhow::{Result, anyhow};
use image::{DynamicImage, ImageBuffer, Rgba};
use rawler::{
    decoders::{Orientation, RawDecodeParams},
    imgop::develop::{DemosaicAlgorithm, Intermediate, ProcessingStep, RawDevelop},
    imgop::xyz::Illuminant,
    rawimage::{RawImage, RawPhotometricInterpretation},
    rawsource::RawSource,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub fn develop_raw_image(
    file_bytes: &[u8],
    fast_demosaic: bool,
    highlight_compression: f32,
    linear_mode: String,
    cancel_token: Option<(Arc<AtomicUsize>, usize)>,
) -> Result<DynamicImage> {
    let (developed_image, orientation) = develop_internal(
        file_bytes,
        fast_demosaic,
        highlight_compression,
        linear_mode,
        cancel_token,
    )?;
    Ok(apply_orientation(developed_image, orientation))
}

fn is_linear_raw_format(raw_image: &RawImage) -> bool {
    matches!(
        raw_image.photometric,
        RawPhotometricInterpretation::LinearRaw
    )
}

/// Nominal temperature of a DNG illuminant code.
///
/// Returns None where the code names no particular light, in which case the
/// matrix it labels cannot take part in interpolation.
fn illuminant_cct(illuminant: Illuminant) -> Option<f32> {
    Some(match illuminant {
        Illuminant::A | Illuminant::Tungsten => 2856.0,
        Illuminant::B => 4874.0,
        Illuminant::C => 6774.0,
        Illuminant::D50 => 5003.0,
        Illuminant::D55 | Illuminant::Daylight | Illuminant::FineWeather | Illuminant::Flash => {
            5503.0
        }
        Illuminant::D65 | Illuminant::CloudyWeather => 6504.0,
        Illuminant::D75 | Illuminant::Shade => 7504.0,
        Illuminant::IsoStudioTungsten => 3200.0,
        Illuminant::Fluorescent | Illuminant::CoolWhiteFluorescent => 4230.0,
        Illuminant::DaylightFluorescent => 6430.0,
        Illuminant::DaylightWhiteFluorescent => 5000.0,
        Illuminant::WhiteFluorescent => 3450.0,
        Illuminant::Unknown => return None,
    })
}

fn to_mat3(coefficients: &[f32]) -> Mat3 {
    let mut m: Mat3 = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = coefficients[i * 3 + j];
        }
    }
    m
}

/// Builds the camera-native to linear ProPhoto transform for this image.
///
/// Cameras are characterised at two reference illuminants, and using either one
/// alone renders anything lit differently with the wrong colorimetry. Where both
/// are present they are interpolated at the frame's own temperature.
///
/// Returns None when the file carries no usable 3x3 colour matrix, in which case
/// the caller leaves rawler's own sRGB calibration in place rather than
/// rendering the image wrong.
fn camera_to_working_space(raw_image: &RawImage, wb_coeffs: [f32; 4]) -> Option<Mat3> {
    // A four-colour sensor carries twelve coefficients and demosaics to a
    // four-channel intermediate that no 3x3 can resolve. Truncating to the
    // first nine would build a plausible-looking matrix for the wrong sensor.
    let usable: Vec<(&Illuminant, &Vec<f32>)> = raw_image
        .color_matrix
        .iter()
        .filter(|(_, coefficients)| coefficients.len() == 9)
        .collect();

    let mut referenced: Vec<(f32, Mat3)> = usable
        .iter()
        .filter_map(|(illuminant, coefficients)| {
            illuminant_cct(**illuminant).map(|cct| (cct, to_mat3(coefficients)))
        })
        .collect();
    referenced.sort_by(|a, b| a.0.total_cmp(&b.0));

    let xyz2cam = match referenced.len() {
        0 => to_mat3(usable.first()?.1),
        1 => referenced[0].1,
        _ => {
            let warm = referenced[0];
            let cool = referenced[referenced.len() - 1];
            crate::white_balance::interpolate_camera_matrix(warm, cool, wb_coeffs)?
        }
    };
    color_space::cam_to_prophoto(xyz2cam)
}

#[inline]
fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn develop_internal(
    file_bytes: &[u8],
    fast_demosaic: bool,
    highlight_compression: f32,
    linear_mode: String,
    cancel_token: Option<(Arc<AtomicUsize>, usize)>,
) -> Result<(DynamicImage, Orientation)> {
    let check_cancel = || -> Result<()> {
        if let Some((tracker, generation)) = &cancel_token
            && tracker.load(Ordering::SeqCst) != *generation
        {
            return Err(anyhow!("Load cancelled"));
        }
        Ok(())
    };

    check_cancel()?;

    let source = RawSource::new_from_slice(file_bytes);
    let decoder = rawler::get_decoder(&source)?;

    check_cancel()?;
    let mut raw_image: RawImage = decoder.raw_image(&source, &RawDecodeParams::default(), false)?;

    let metadata = decoder.raw_metadata(&source, &RawDecodeParams::default())?;
    let orientation = metadata
        .exif
        .orientation
        .map(Orientation::from_u16)
        .unwrap_or(Orientation::Normal);

    let is_linear_format = is_linear_raw_format(&raw_image);

    let (apply_ungamma, apply_calibration) = match linear_mode.as_str() {
        "gamma" => (true, true),
        "skip_calib" => (false, false),
        "gamma_skip_calib" => (true, false),
        _ => (false, true),
    };

    let original_white_level = raw_image
        .whitelevel
        .0
        .first()
        .cloned()
        .unwrap_or(u16::MAX as u32) as f32;
    let original_black_level = raw_image
        .blacklevel
        .levels
        .first()
        .map(|r| r.as_f32())
        .unwrap_or(0.0);

    for level in raw_image.whitelevel.0.iter_mut() {
        *level = u32::MAX;
    }

    raw_image.wb_coeffs =
        crate::multi_exposure::neutralize_wb_if_multiexposure(raw_image.wb_coeffs, file_bytes);

    let wb = if raw_image.wb_coeffs[0].is_nan() {
        [1.0f32; 4]
    } else {
        raw_image.wb_coeffs
    };

    let working_transform = if apply_calibration {
        camera_to_working_space(&raw_image, wb)
    } else {
        None
    };

    let mut developer = RawDevelop::default();

    if is_linear_format {
        developer.steps.retain(|&step| {
            step != ProcessingStep::SRgb
                && step != ProcessingStep::Demosaic
                && (apply_calibration || step != ProcessingStep::Calibrate)
        });
    } else if fast_demosaic {
        developer.demosaic_algorithm = DemosaicAlgorithm::Speed;
        developer.steps.retain(|&step| step != ProcessingStep::SRgb);
    } else {
        developer.steps.retain(|&step| step != ProcessingStep::SRgb);
    }

    // rawler's Calibrate resolves to sRGB primaries and clips there, discarding
    // every colour the sensor recorded outside that gamut before any adjustment
    // runs. Where we have our own transform we take the camera-native data.
    if working_transform.is_some() {
        developer
            .steps
            .retain(|&step| step != ProcessingStep::Calibrate);
    }

    // Whatever we did not calibrate ourselves, rawler resolved to sRGB. Lifting
    // that into the working space below keeps exactly one space downstream.
    let rawler_calibrated = developer.steps.contains(&ProcessingStep::Calibrate);

    check_cancel()?;
    let mut developed_intermediate = developer.develop_intermediate(&raw_image)?;

    drop(raw_image);

    let denominator = (original_white_level - original_black_level).max(1.0);
    let rescale_factor = (u32::MAX as f32 - original_black_level) / denominator;

    let safe_highlight_compression = highlight_compression.max(1.01);

    let clamp_limit = if fast_demosaic {
        1.0
    } else {
        safe_highlight_compression
    };

    check_cancel()?;

    match &mut developed_intermediate {
        Intermediate::Monochrome(pixels) => {
            pixels.data.iter_mut().for_each(|p| {
                let mut linear_val = *p * rescale_factor;
                if is_linear_format && apply_ungamma {
                    linear_val = srgb_to_linear(linear_val.clamp(0.0, 1.0));
                }
                *p = linear_val.clamp(0.0, clamp_limit);
            });
        }
        Intermediate::ThreeColor(pixels) => {
            pixels.data.iter_mut().for_each(|p| {
                let mut r = (p[0] * rescale_factor).max(0.0);
                let mut g = (p[1] * rescale_factor).max(0.0);
                let mut b = (p[2] * rescale_factor).max(0.0);

                if is_linear_format && apply_ungamma {
                    r = srgb_to_linear(r.clamp(0.0, 1.0));
                    g = srgb_to_linear(g.clamp(0.0, 1.0));
                    b = srgb_to_linear(b.clamp(0.0, 1.0));
                }

                let working = if let Some(matrix) = working_transform {
                    Some(color_space::apply(
                        &matrix,
                        [r * wb[0], g * wb[1], b * wb[2]],
                    ))
                } else if rawler_calibrated {
                    Some(color_space::srgb_to_prophoto([r, g, b]))
                } else {
                    None
                };

                if let Some(working) = working {
                    // The residual out-of-gamut fraction in ProPhoto is a small
                    // fraction of a percent, and letting it stay negative would
                    // reach operators that take sqrt and pow of their input.
                    r = working[0].max(0.0);
                    g = working[1].max(0.0);
                    b = working[2].max(0.0);
                }

                let max_c = r.max(g).max(b);

                let (final_r, final_g, final_b) = if max_c > 1.0 {
                    let min_c = r.min(g).min(b);
                    let compression_factor =
                        (1.0 - (max_c - 1.0) / (safe_highlight_compression - 1.0)).clamp(0.0, 1.0);
                    let compressed_r = min_c + (r - min_c) * compression_factor;
                    let compressed_g = min_c + (g - min_c) * compression_factor;
                    let compressed_b = min_c + (b - min_c) * compression_factor;
                    let compressed_max = compressed_r.max(compressed_g).max(compressed_b);

                    if compressed_max > 1e-6 {
                        let rescale = max_c / compressed_max;
                        (
                            compressed_r * rescale,
                            compressed_g * rescale,
                            compressed_b * rescale,
                        )
                    } else {
                        (max_c, max_c, max_c)
                    }
                } else {
                    (r, g, b)
                };

                p[0] = final_r.clamp(0.0, clamp_limit);
                p[1] = final_g.clamp(0.0, clamp_limit);
                p[2] = final_b.clamp(0.0, clamp_limit);
            });
        }
        Intermediate::FourColor(pixels) => {
            pixels.data.iter_mut().for_each(|p| {
                p.iter_mut().for_each(|c| {
                    let mut linear_val = *c * rescale_factor;
                    if is_linear_format && apply_ungamma {
                        linear_val = srgb_to_linear(linear_val.clamp(0.0, 1.0));
                    }
                    *c = linear_val.clamp(0.0, clamp_limit);
                });
            });
        }
    }

    let (width, height) = {
        let dim = developed_intermediate.dim();
        (dim.w as u32, dim.h as u32)
    };

    check_cancel()?;

    let dynamic_image = match developed_intermediate {
        Intermediate::ThreeColor(pixels) => {
            let buffer = ImageBuffer::<Rgba<f32>, _>::from_fn(width, height, |x, y| {
                let p = pixels.data[(y * width + x) as usize];
                Rgba([p[0], p[1], p[2], 1.0])
            });
            DynamicImage::ImageRgba32F(buffer)
        }
        Intermediate::Monochrome(pixels) => {
            let buffer = ImageBuffer::<Rgba<f32>, _>::from_fn(width, height, |x, y| {
                let p = pixels.data[(y * width + x) as usize];
                Rgba([p, p, p, 1.0])
            });
            DynamicImage::ImageRgba32F(buffer)
        }
        _ => {
            return Err(anyhow!("Unsupported intermediate format for conversion"));
        }
    };

    Ok((dynamic_image, orientation))
}

pub fn get_fast_demosaic_scale_factor(
    file_bytes: &[u8],
    decoded_width: u32,
    decoded_height: u32,
) -> f32 {
    let source = RawSource::new_from_slice(file_bytes);
    if let Ok(decoder) = rawler::get_decoder(&source)
        && let Ok(raw_img) = decoder.raw_image(&source, &RawDecodeParams::default(), true)
    {
        let max_orig = (raw_img.width as f32).max(raw_img.height as f32);
        let max_comp = (decoded_width as f32).max(decoded_height as f32);
        if max_orig > 0.0 {
            let ratio = max_comp / max_orig;
            if ratio > 0.1 && ratio < 0.35 {
                return 0.25;
            } else if (0.35..0.75).contains(&ratio) {
                return 0.5;
            }
        }
    }
    1.0
}
