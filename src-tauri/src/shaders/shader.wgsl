struct Point {
    x: f32,
    y: f32,
    _pad1: f32,
    _pad2: f32,
}

struct HslColor {
    hue: f32,
    saturation: f32,
    luminance: f32,
    _pad: f32,
}

struct ColorGradeSettings {
    hue: f32,
    saturation: f32,
    luminance: f32,
    _pad: f32,
}

struct ColorCalibrationSettings {
    shadows_tint: f32,
    red_hue: f32,
    red_saturation: f32,
    green_hue: f32,
    green_saturation: f32,
    blue_hue: f32,
    blue_saturation: f32,
    _pad1: f32,
}

struct GlobalAdjustments {
    exposure: f32,
    brightness: f32,
    contrast: f32,
    highlights: f32,
    shadows: f32,
    whites: f32,
    blacks: f32,
    saturation: f32,
    temperature: f32,
    tint: f32,
    vibrance: f32,
    hue: f32,
    color_mixer_preview: f32,
    _pad_color2: f32,
    _pad_color3: f32,

    sharpness: f32,
    luma_noise_reduction: f32,
    color_noise_reduction: f32,
    clarity: f32,
    dehaze: f32,
    structure: f32,
    centre: f32,
    vignette_amount: f32,
    vignette_midpoint: f32,
    vignette_roundness: f32,
    vignette_feather: f32,
    grain_amount: f32,
    grain_size: f32,
    grain_roughness: f32,

    chromatic_aberration_red_cyan: f32,
    chromatic_aberration_blue_yellow: f32,
    show_clipping: u32,
    is_raw_image: u32,
    _pad_ca1: f32,

    has_lut: u32,
    lut_intensity: f32,
    tonemapper_mode: u32,
    lut_is_scene_referred: u32,
    _pad_lut3: f32,
    _pad_lut4: f32,
    _pad_lut5: f32,

    _pad_agx1: f32,
    _pad_agx2: f32,
    _pad_agx3: f32,
    agx_pipe_to_rendering_matrix: mat3x3<f32>,
    agx_rendering_to_pipe_matrix: mat3x3<f32>,

    _pad_cg1: f32,
    _pad_cg2: f32,
    _pad_cg3: f32,
    _pad_cg4: f32,
    color_grading_shadows: ColorGradeSettings,
    color_grading_midtones: ColorGradeSettings,
    color_grading_highlights: ColorGradeSettings,
    color_grading_global: ColorGradeSettings,
    color_grading_blending: f32,
    color_grading_balance: f32,
    _pad2: f32,
    _pad3: f32,

    color_calibration: ColorCalibrationSettings,

    hsl: array<HslColor, 8>,
    luma_curve: array<Point, 16>,
    red_curve: array<Point, 16>,
    green_curve: array<Point, 16>,
    blue_curve: array<Point, 16>,
    luma_curve_count: u32,
    red_curve_count: u32,
    green_curve_count: u32,
    blue_curve_count: u32,
    _pad_end1: f32,
    _pad_end2: f32,
    _pad_end3: f32,
    _pad_end4: f32,

    glow_amount: f32,
    halation_amount: f32,
    flare_amount: f32,
    sharpness_threshold: f32,
}

struct MaskAdjustments {
    exposure: f32,
    brightness: f32,
    contrast: f32,
    highlights: f32,
    shadows: f32,
    whites: f32,
    blacks: f32,
    saturation: f32,
    temperature: f32,
    tint: f32,
    vibrance: f32,

    sharpness: f32,
    luma_noise_reduction: f32,
    color_noise_reduction: f32,
    clarity: f32,
    dehaze: f32,
    structure: f32,

    glow_amount: f32,
    halation_amount: f32,
    flare_amount: f32,
    sharpness_threshold: f32,

    hue: f32,
    _pad_cg1: f32,
    _pad_cg2: f32,
    color_grading_shadows: ColorGradeSettings,
    color_grading_midtones: ColorGradeSettings,
    color_grading_highlights: ColorGradeSettings,
    color_grading_global: ColorGradeSettings,
    color_grading_blending: f32,
    color_grading_balance: f32,
    _pad5: f32,
    _pad6: f32,

    hsl: array<HslColor, 8>,
    luma_curve: array<Point, 16>,
    red_curve: array<Point, 16>,
    green_curve: array<Point, 16>,
    blue_curve: array<Point, 16>,
    luma_curve_count: u32,
    red_curve_count: u32,
    green_curve_count: u32,
    blue_curve_count: u32,
    _pad_end4: f32,
    _pad_end5: f32,
    _pad_end6: f32,
    _pad_end7: f32,
}

struct AllAdjustments {
    global: GlobalAdjustments,
    mask_adjustments: array<MaskAdjustments, 32>,
    mask_count: u32,
    tile_offset_x: u32,
    tile_offset_y: u32,
    mask_atlas_cols: u32,
}

struct HslRange {
    center: f32,
    width: f32,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<storage, read> adjustments: AllAdjustments;

@group(0) @binding(3) var mask_textures: texture_2d_array<f32>;

@group(0) @binding(4) var lut_texture: texture_3d<f32>;
@group(0) @binding(5) var lut_sampler: sampler;

@group(0) @binding(6) var sharpness_blur_texture: texture_2d<f32>;
@group(0) @binding(7) var tonal_blur_texture: texture_2d<f32>;
@group(0) @binding(8) var clarity_blur_texture: texture_2d<f32>;
@group(0) @binding(9) var structure_blur_texture: texture_2d<f32>;

@group(0) @binding(10) var flare_texture: texture_2d<f32>;
@group(0) @binding(11) var flare_sampler: sampler;

// Relative luminance for the ProPhoto working space. Rec.709 weights would
// misweight every luma-driven operator now that the pipeline is not sRGB.
const LUMA_COEFF = vec3<f32>(0.2880402, 0.7118741, 0.0000857);
const SRGB_LUMA = vec3<f32>(0.2126, 0.7152, 0.0722);

// Rows of the linear ProPhoto (D50) to linear sRGB (D65) Bradford-adapted
// transform and its inverse, generated from color_space.rs.
const PROPHOTO_TO_SRGB_R0 = vec3<f32>(2.0340760, -0.7273341, -0.3067416);
const PROPHOTO_TO_SRGB_R1 = vec3<f32>(-0.2288132, 1.2317301, -0.0029170);
const PROPHOTO_TO_SRGB_R2 = vec3<f32>(-0.0085698, -0.1532867, 1.1618567);

const SRGB_TO_PROPHOTO_R0 = vec3<f32>(0.5293459, 0.3300728, 0.1405812);
const SRGB_TO_PROPHOTO_R1 = vec3<f32>(0.0983743, 0.8734611, 0.0281647);
const SRGB_TO_PROPHOTO_R2 = vec3<f32>(0.0168832, 0.1176725, 0.8654441);

/// Correlated colour temperature of the working space white, which is the
/// illuminant the shot's own neutral was pinned to during develop.
const WB_ORIGIN_CCT: f32 = 5003.0;
/// Mireds per unit of travel. Reciprocal temperature rather than kelvin, so a
/// step feels the same at both ends of the range. These absorb the divisors the
/// adjustment plumbing applies, so they are the per-slider values times the
/// scales in white_balance.rs, which asserts the relationship.
const WB_MIREDS_PER_STEP: f32 = 37.5;
const WB_TINT_V_PER_STEP: f32 = 0.03;

const PP_TO_CONE = mat3x3<f32>(
    vec3<f32>(0.7907327, -0.1048588, 0.0112988),
    vec3<f32>(0.3106534, 1.1183755, -0.0435044),
    vec3<f32>(-0.1051016, 0.0069107, 0.8508500),
);
const CONE_TO_PP = mat3x3<f32>(
    vec3<f32>(1.2183720, 0.1142984, -0.0103351),
    vec3<f32>(-0.3324701, 0.8626819, 0.0485244),
    vec3<f32>(0.1532003, 0.0071119, 1.1736245),
);
const BRADFORD = mat3x3<f32>(
    vec3<f32>(0.8951000, -0.7502000, 0.0389000),
    vec3<f32>(0.2664000, 1.7135000, -0.0685000),
    vec3<f32>(-0.1614000, 0.0367000, 1.0296000),
);

fn get_luma(c: vec3<f32>) -> f32 {
    return dot(c, LUMA_COEFF);
}

/// Luminance for data that has already left the working space through the
/// display transform, where ProPhoto weights would misweight blue by ~840x.
fn get_display_luma(c: vec3<f32>) -> f32 {
    return dot(c, SRGB_LUMA);
}

/// Brings a sample of input_texture into the working space. Raw input is
/// already there; non-raw input is sRGB-encoded, exactly as main assumes for
/// the primary sample.
fn input_to_working(c: vec3<f32>) -> vec3<f32> {
    return srgb_to_prophoto(srgb_to_linear(c));
}

fn prophoto_to_srgb(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(PROPHOTO_TO_SRGB_R0, c),
        dot(PROPHOTO_TO_SRGB_R1, c),
        dot(PROPHOTO_TO_SRGB_R2, c),
    );
}

fn srgb_to_prophoto(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(SRGB_TO_PROPHOTO_R0, c),
        dot(SRGB_TO_PROPHOTO_R1, c),
        dot(SRGB_TO_PROPHOTO_R2, c),
    );
}

/// Desaturates a colour toward its own luminance until its darkest channel
/// reaches zero, so a colour outside sRGB loses saturation rather than the hue
/// and luminance a per-channel clamp would shift.
fn gamut_clip_srgb(c: vec3<f32>) -> vec3<f32> {
    let min_c = min(c.r, min(c.g, c.b));
    if (min_c >= 0.0) {
        return c;
    }
    let luma = dot(c, SRGB_LUMA);
    if (luma <= 0.0) {
        return vec3<f32>(0.0);
    }
    return mix(vec3<f32>(luma), c, luma / (luma - min_c));
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.04045);
    let a = vec3<f32>(0.055);
    let higher = pow((c + a) / (1.0 + a), vec3<f32>(2.4));
    let lower = c / 12.92;
    return select(higher, lower, c <= cutoff);
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let c_clamped = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let cutoff = vec3<f32>(0.0031308);
    let a = vec3<f32>(0.055);
    let higher = (1.0 + a) * pow(c_clamped, vec3<f32>(1.0 / 2.4)) - a;
    let lower = c_clamped * 12.92;
    return select(higher, lower, c_clamped <= cutoff);
}

fn linear_to_srgb_extended(c: vec3<f32>) -> vec3<f32> {
    let safe_c = max(c, vec3<f32>(0.0));
    let cutoff = vec3<f32>(0.0031308);
    let a = vec3<f32>(0.055);
    let higher = (1.0 + a) * pow(safe_c, vec3<f32>(1.0 / 2.4)) - a;
    let lower = safe_c * 12.92;
    return select(higher, lower, safe_c <= cutoff);
}

fn linear_to_vlog(c: vec3<f32>) -> vec3<f32> {
    let safe_c = max(c, vec3<f32>(0.0));
    let low = 5.6 * safe_c + 0.125;
    let log10_val = log2(safe_c + 0.00873) * 0.30102999566398;
    let high = 0.241514 * log10_val + 0.598206;
    return select(high, low, safe_c <= vec3<f32>(0.01));
}

fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let c_max = max(c.r, max(c.g, c.b));
    let c_min = min(c.r, min(c.g, c.b));
    let delta = c_max - c_min;
    var h: f32 = 0.0;
    if (delta > 0.0) {
        if (c_max == c.r) { h = 60.0 * (((c.g - c.b) / delta) % 6.0); }
        else if (c_max == c.g) { h = 60.0 * (((c.b - c.r) / delta) + 2.0); }
        else { h = 60.0 * (((c.r - c.g) / delta) + 4.0); }
    }
    if (h < 0.0) { h += 360.0; }
    let s = select(0.0, delta / c_max, c_max > 0.0);
    return vec3<f32>(h, s, c_max);
}

fn hsv_to_rgb(c: vec3<f32>) -> vec3<f32> {
    let h = c.x; let s = c.y; let v = c.z;
    let C = v * s;
    let X = C * (1.0 - abs((h / 60.0) % 2.0 - 1.0));
    let m = v - C;
    var rgb_prime: vec3<f32>;
    if (h < 60.0) { rgb_prime = vec3<f32>(C, X, 0.0); }
    else if (h < 120.0) { rgb_prime = vec3<f32>(X, C, 0.0); }
    else if (h < 180.0) { rgb_prime = vec3<f32>(0.0, C, X); }
    else if (h < 240.0) { rgb_prime = vec3<f32>(0.0, X, C); }
    else if (h < 300.0) { rgb_prime = vec3<f32>(X, 0.0, C); }
    else { rgb_prime = vec3<f32>(C, 0.0, X); }
    return rgb_prime + vec3<f32>(m, m, m);
}

fn hash(p: vec2<f32>) -> f32 {
    var p3  = fract(vec3<f32>(p.xyx) * .1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn gradient_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    let ga = vec2<f32>(hash(i + vec2(0.0, 0.0)), hash(i + vec2(0.0, 0.0) + vec2(11.0, 37.0))) * 2.0 - 1.0;
    let gb = vec2<f32>(hash(i + vec2(1.0, 0.0)), hash(i + vec2(1.0, 0.0) + vec2(11.0, 37.0))) * 2.0 - 1.0;
    let gc = vec2<f32>(hash(i + vec2(0.0, 1.0)), hash(i + vec2(0.0, 1.0) + vec2(11.0, 37.0))) * 2.0 - 1.0;
    let gd = vec2<f32>(hash(i + vec2(1.0, 1.0)), hash(i + vec2(1.0, 1.0) + vec2(11.0, 37.0))) * 2.0 - 1.0;

    let dot_00 = dot(ga, f - vec2(0.0, 0.0));
    let dot_10 = dot(gb, f - vec2(1.0, 0.0));
    let dot_01 = dot(gc, f - vec2(0.0, 1.0));
    let dot_11 = dot(gd, f - vec2(1.0, 1.0));

    let bottom_interp = mix(dot_00, dot_10, u.x);
    let top_interp = mix(dot_01, dot_11, u.x);

    return mix(bottom_interp, top_interp, u.y);
}

fn dither(coords: vec2<u32>) -> f32 {
    let p = vec2<f32>(coords);
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5;
}

fn interpolate_cubic_hermite(x: f32, p1: Point, p2: Point, m1: f32, m2: f32) -> f32 {
    let dx = p2.x - p1.x;
    if (dx <= 0.0) { return p1.y; }
    let t = (x - p1.x) / dx;
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    return h00 * p1.y + h10 * m1 * dx + h01 * p2.y + h11 * m2 * dx;
}

fn apply_curve(val: f32, points: array<Point, 16>, count: u32) -> f32 {
    if (count < 2u) { return val; }
    var local_points = points;
    let x = val * 255.0;
    if (x <= local_points[0].x) { return local_points[0].y / 255.0; }
    if (x >= local_points[count - 1u].x) { return local_points[count - 1u].y / 255.0; }
    for (var i = 0u; i < 15u; i = i + 1u) {
        if (i >= count - 1u) { break; }
        let p1 = local_points[i];
        let p2 = local_points[i + 1u];
        if (x <= p2.x) {
            let p0 = local_points[max(0u, i - 1u)];
            let p3 = local_points[min(count - 1u, i + 2u)];
            let delta_before = (p1.y - p0.y) / max(0.001, p1.x - p0.x);
            let delta_current = (p2.y - p1.y) / max(0.001, p2.x - p1.x);
            let delta_after = (p3.y - p2.y) / max(0.001, p3.x - p2.x);
            var tangent_at_p1: f32;
            var tangent_at_p2: f32;
            if (i == 0u) { tangent_at_p1 = delta_current; } else {
                if (delta_before * delta_current <= 0.0) { tangent_at_p1 = 0.0; } else { tangent_at_p1 = (delta_before + delta_current) / 2.0; }
            }
            if (i + 1u == count - 1u) { tangent_at_p2 = delta_current; } else {
                if (delta_current * delta_after <= 0.0) { tangent_at_p2 = 0.0; } else { tangent_at_p2 = (delta_current + delta_after) / 2.0; }
            }
            if (delta_current != 0.0) {
                let alpha = tangent_at_p1 / delta_current;
                let beta = tangent_at_p2 / delta_current;
                if (alpha * alpha + beta * beta > 9.0) {
                    let tau = 3.0 / sqrt(alpha * alpha + beta * beta);
                    tangent_at_p1 = tangent_at_p1 * tau;
                    tangent_at_p2 = tangent_at_p2 * tau;
                }
            }
            let result_y = interpolate_cubic_hermite(x, p1, p2, tangent_at_p1, tangent_at_p2);
            return clamp(result_y / 255.0, 0.0, 1.0);
        }
    }
    return local_points[count - 1u].y / 255.0;
}

fn apply_tonal_adjustments(
    color: vec3<f32>,
    blurred_color_input_space: vec3<f32>,
    is_raw: u32,
    con: f32,
    sh: f32,
    wh: f32,
    bl: f32
) -> vec3<f32> {
    var rgb = color;

    var blurred_linear: vec3<f32>;
    if (is_raw == 1u) {
        blurred_linear = blurred_color_input_space;
    } else {
        blurred_linear = input_to_working(blurred_color_input_space);
    }

    if (wh != 0.0) {
        let white_level = 1.0 - wh * 0.25;
        let w_mult = 1.0 / max(white_level, 0.01);
        rgb *= w_mult;
        blurred_linear *= w_mult;
    }

    let pixel_luma = get_luma(max(rgb, vec3<f32>(0.0)));
    let blurred_luma = get_luma(max(blurred_linear, vec3<f32>(0.0)));

    let safe_pixel_luma = max(pixel_luma, 0.0001);
    let safe_blurred_luma = max(blurred_luma, 0.0001);

    if (sh != 0.0 || bl != 0.0) {
        let t_pixel = pow(safe_pixel_luma, 0.4545);
        let t_blurred = pow(safe_blurred_luma, 0.4545);

        let shadow_lift = sh * t_pixel * pow(max(1.0 - t_pixel, 0.0), 4.5);
        let black_lift = bl * t_pixel * pow(max(1.0 - t_pixel, 0.0), 12.0);
        let lift_amount = max(shadow_lift + black_lift, 0.0);

        let t_pixel_curved = max(t_pixel + shadow_lift + black_lift, 0.0);

        let shadow_pivot = 0.2;
        let stretch_factor = 1.0 + (lift_amount * 1.3);
        let contrasted_t = shadow_pivot + (t_pixel_curved - shadow_pivot) * stretch_factor;

        let final_t = max(mix(t_pixel_curved, contrasted_t, 0.85), 0.0);
        let curved_luma = pow(final_t, 2.2);

        let luma_ratio = curved_luma / safe_pixel_luma;
        rgb *= luma_ratio;

        let detail = t_pixel / max(t_blurred, 0.0001);
        let safe_detail = clamp(detail, 0.8, 1.25);

        let noise_protection = smoothstep(0.0, 0.1, t_blurred);

        let detail_amp = 1.0 + (lift_amount * 1.2 * noise_protection);

        let enhanced_detail = pow(safe_detail, detail_amp);
        let detail_correction = enhanced_detail / safe_detail;

        let linear_correction = pow(detail_correction, 2.2);
        rgb *= linear_correction;

        if (luma_ratio > 1.0) {
            let recovered_luma = get_luma(rgb);
            let boost_amount = clamp((luma_ratio - 1.0) * 0.15, 0.0, 0.4);
            rgb = mix(rgb, vec3<f32>(recovered_luma), boost_amount);
        }
    }

    if (con != 0.0) {
        let safe_rgb = max(rgb, vec3<f32>(0.0));
        let g = 2.2;
        let perceptual = pow(safe_rgb, vec3<f32>(1.0 / g));
        let clamped_perceptual = clamp(perceptual, vec3<f32>(0.0), vec3<f32>(1.0));
        let strength = pow(2.0, con * 1.25);
        let condition = clamped_perceptual < vec3<f32>(0.5);
        let high_part = 1.0 - 0.5 * pow(2.0 * (1.0 - clamped_perceptual), vec3<f32>(strength));
        let low_part = 0.5 * pow(2.0 * clamped_perceptual, vec3<f32>(strength));
        let curved_perceptual = select(high_part, low_part, condition);
        let contrast_adjusted_rgb = pow(curved_perceptual, vec3<f32>(g));
        let mix_factor = smoothstep(vec3<f32>(1.0), vec3<f32>(1.01), safe_rgb);
        rgb = mix(contrast_adjusted_rgb, rgb, mix_factor);
    }
    return rgb;
}

fn apply_highlights_adjustment(
    color_in: vec3<f32>,
    blurred_color_input_space: vec3<f32>,
    is_raw: u32,
    highlights_adj: f32
) -> vec3<f32> {
    if (highlights_adj == 0.0) { return color_in; }

    let pixel_luma = get_luma(max(color_in, vec3<f32>(0.0)));
    let safe_pixel_luma = max(pixel_luma, 0.0001);

    let pixel_mask_input = tanh(safe_pixel_luma * 1.5);
    let highlight_mask = smoothstep(0.3, 0.95, pixel_mask_input);

    if (highlight_mask < 0.001) {
        return color_in;
    }

    let luma = pixel_luma;
    var final_adjusted_color: vec3<f32>;

    if (highlights_adj < 0.0) {
        var new_luma: f32;
        if (luma <= 1.0) {
            let gamma = 1.0 - highlights_adj * 1.75;
            new_luma = pow(luma, gamma);
        } else {
            let luma_excess = luma - 1.0;
            let compression_strength = -highlights_adj * 6.0;
            let compressed_excess = luma_excess / (1.0 + luma_excess * compression_strength);
            new_luma = 1.0 + compressed_excess;
        }
        let tonally_adjusted_color = color_in * (new_luma / max(luma, 0.0001));
        let desaturation_amount = smoothstep(1.0, 10.0, luma);
        let white_point = vec3<f32>(new_luma);
        final_adjusted_color = mix(tonally_adjusted_color, white_point, desaturation_amount);
    } else {
        let adjustment = highlights_adj * 1.75;
        let factor = pow(2.0, adjustment);
        final_adjusted_color = color_in * factor;
    }

    return mix(color_in, final_adjusted_color, highlight_mask);
}

fn apply_linear_exposure(color_in: vec3<f32>, exposure_adj: f32) -> vec3<f32> {
    if (exposure_adj == 0.0) {
        return color_in;
    }
    return color_in * pow(2.0, exposure_adj);
}

fn apply_filmic_exposure(color_in: vec3<f32>, brightness_adj: f32) -> vec3<f32> {
    if (brightness_adj == 0.0) {
        return color_in;
    }
    const RATIONAL_CURVE_MIX: f32 = 0.95;
    const MIDTONE_STRENGTH: f32 = 1.2;
    const TOP_ANCHOR: f32 = 1.06;
    let original_luma = get_luma(color_in);
    if (abs(original_luma) < 0.00001) {
        return color_in;
    }
    let direct_adj = brightness_adj * (1.0 - RATIONAL_CURVE_MIX);
    let rational_adj = brightness_adj * RATIONAL_CURVE_MIX;
    let scale = pow(2.0, direct_adj);
    let k = pow(2.0, -rational_adj * MIDTONE_STRENGTH);
    let luma_abs = abs(original_luma);
    let luma_floor = floor(luma_abs / TOP_ANCHOR) * TOP_ANCHOR;
    let luma_norm = (luma_abs - luma_floor) / TOP_ANCHOR;
    let shaped_norm = luma_norm / (luma_norm + (1.0 - luma_norm) * k);
    let shaped_luma_abs = luma_floor + (shaped_norm * TOP_ANCHOR);
    let new_luma = sign(original_luma) * shaped_luma_abs * scale;
    let chroma = color_in - vec3<f32>(original_luma);
    let total_luma_scale = new_luma / original_luma;
    let luma_weight = clamp(new_luma, 0.0, 2.0) * 0.5;
    let dynamic_exp = mix(0.95, 0.65, luma_weight);
    let base_chroma_scale = pow(total_luma_scale, dynamic_exp);
    let highlight_rolloff = 1.0 / (1.0 + max(0.0, new_luma - 0.9) * 2.0);
    let chroma_scale = base_chroma_scale * highlight_rolloff;
    return vec3<f32>(new_luma) + chroma * chroma_scale;
}

fn apply_color_calibration(color: vec3<f32>, cal: ColorCalibrationSettings) -> vec3<f32> {
    let h_r = cal.red_hue;
    let h_g = cal.green_hue;
    let h_b = cal.blue_hue;
    let r_prime = vec3<f32>(1.0 - abs(h_r), max(0.0, h_r), max(0.0, -h_r));
    let g_prime = vec3<f32>(max(0.0, -h_g), 1.0 - abs(h_g), max(0.0, h_g));
    let b_prime = vec3<f32>(max(0.0, h_b), max(0.0, -h_b), 1.0 - abs(h_b));
    let hue_matrix = mat3x3<f32>(r_prime, g_prime, b_prime);
    var c = hue_matrix * color;

    let luma = get_luma(max(vec3(0.0), c));
    let desaturated_color = vec3<f32>(luma);
    let sat_vector = c - desaturated_color;

    let color_sum = c.r + c.g + c.b;
    var masks = vec3<f32>(0.0);
    if (color_sum > 0.001) {
        masks = c / color_sum;
    }

    let total_sat_adjustment =
        masks.r * cal.red_saturation +
        masks.g * cal.green_saturation +
        masks.b * cal.blue_saturation;

    c += sat_vector * total_sat_adjustment;

    let st = cal.shadows_tint;
    if (abs(st) > 0.001) {
        let shadow_luma = get_luma(max(vec3(0.0), c));
        let mask = 1.0 - smoothstep(0.0, 0.3, shadow_luma);
        let tint_mult = vec3<f32>(1.0 + st * 0.25, 1.0 - st * 0.25, 1.0 + st * 0.25);
        c = mix(c, c * tint_mult, mask);
    }

    return c;
}

/// Chromaticity of the illuminant at a temperature, on the Planckian locus
/// below 4000 K where real sources are incandescent and the CIE daylight locus
/// above it, following the DNG convention.
fn wb_xy_from_cct(cct: f32) -> vec2<f32> {
    let t = clamp(cct, 1000.0, 50000.0);
    let inv = 1.0e3 / t;
    let inv2 = inv * inv;
    let inv3 = inv2 * inv;

    var x: f32;
    var y: f32;
    if (t < 4000.0) {
        if (t <= 2222.0) {
            x = -0.2661239 * inv3 - 0.2343589 * inv2 + 0.8776956 * inv + 0.179910;
            y = -1.1063814 * x * x * x - 1.34811020 * x * x + 2.18555832 * x - 0.20219683;
        } else {
            x = -3.0258469 * inv3 + 2.1070379 * inv2 + 0.2226347 * inv + 0.240390;
            y = -0.9549476 * x * x * x - 1.37418593 * x * x + 2.09137015 * x - 0.16748867;
        }
    } else {
        if (t <= 7000.0) {
            x = -4.6070 * inv3 + 2.9678 * inv2 + 0.09911 * inv + 0.244063;
        } else {
            x = -2.0064 * inv3 + 1.9018 * inv2 + 0.24748 * inv + 0.237040;
        }
        y = -3.000 * x * x + 2.870 * x - 0.275;
    }
    return vec2<f32>(x, y);
}

fn wb_uv_from_xy(xy: vec2<f32>) -> vec2<f32> {
    let d = -2.0 * xy.x + 12.0 * xy.y + 3.0;
    if (abs(d) < 1e-9) {
        return vec2<f32>(0.0, 0.0);
    }
    return vec2<f32>(4.0 * xy.x / d, 6.0 * xy.y / d);
}

fn wb_xy_from_uv(uv: vec2<f32>) -> vec2<f32> {
    let d = 2.0 * uv.x - 8.0 * uv.y + 4.0;
    if (abs(d) < 1e-9) {
        return vec2<f32>(0.3127, 0.3290);
    }
    return vec2<f32>(3.0 * uv.x / d, 2.0 * uv.y / d);
}

fn wb_xyz_from_xy(xy: vec2<f32>) -> vec3<f32> {
    if (abs(xy.y) < 1e-6) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }
    return vec3<f32>(xy.x / xy.y, 1.0, (1.0 - xy.x - xy.y) / xy.y);
}

/// White balance as a chromatic adaptation rather than a channel scale.
///
/// The working space pins the shot's own neutral to its white, so that white is
/// the origin the sliders move from. Both origin and target run through the same
/// locus approximation, which makes zero exactly identity rather than nearly so.
///
/// Both axes invert: the target illuminant is divided out, so a cooler target is
/// what leaves the image warmer, and a greener one what leaves it magenta.
fn apply_white_balance(color: vec3<f32>, temp: f32, tnt: f32) -> vec3<f32> {
    if (abs(temp) < 0.0005 && abs(tnt) < 0.0005) {
        return color;
    }

    let origin_xy = wb_xy_from_cct(WB_ORIGIN_CCT);

    let target_mireds = clamp(
        1.0e6 / WB_ORIGIN_CCT - temp * WB_MIREDS_PER_STEP,
        20.0,
        1000.0
    );
    let target_uv = wb_uv_from_xy(wb_xy_from_cct(1.0e6 / target_mireds));
    let tinted_xy = wb_xy_from_uv(vec2<f32>(
        target_uv.x,
        target_uv.y + tnt * WB_TINT_V_PER_STEP
    ));

    let origin_cone = BRADFORD * wb_xyz_from_xy(origin_xy);
    let target_cone = BRADFORD * wb_xyz_from_xy(tinted_xy);

    return CONE_TO_PP * ((PP_TO_CONE * color) * (origin_cone / target_cone));
}


// Oklab, generated from oklab.rs which asserts these against this file.
const PP_TO_LMS = mat3x3<f32>(
    vec3<f32>(0.71538717, 0.27443418, 0.10983816),
    vec3<f32>(0.35280859, 0.66782898, 0.18630311),
    vec3<f32>(-0.06826405, 0.05775598, 0.70419478),
);
const LMS_TO_PP = mat3x3<f32>(
    vec3<f32>(1.73857641, -0.70716941, -0.08408777),
    vec3<f32>(-0.98809987, 1.93436372, -0.35763812),
    vec3<f32>(0.24957718, -0.22720321, 1.44124269),
);
const LMS_TO_OKLAB = mat3x3<f32>(
    vec3<f32>(0.21045426, 1.97799850, 0.02590404),
    vec3<f32>(0.79361778, -2.42859221, 0.78277177),
    vec3<f32>(-0.00407205, 0.45059371, -0.80867577),
);
const OKLAB_TO_LMS = mat3x3<f32>(
    vec3<f32>(1.00000000, 1.00000000, 1.00000000),
    vec3<f32>(0.39633778, -0.10556135, -0.08948418),
    vec3<f32>(0.21580376, -0.06385417, -1.29148555),
);

// Oklch hue of each band's canonical colour, so a band selects the colours a
// photographer means by its name rather than the ones an RGB hexcone happens to
// place there. Blue sits at 264 here against 225 in the old HSV geometry.
const OK_BAND_CENTERS = array<f32, 8>(29.23, 67.93, 109.78, 142.51, 194.80, 264.06, 311.99, 328.36);

// Skin holds an Oklch hue within a couple of degrees of this across light and
// deep tones alike, which is what makes guarding it there meaningful.
const OK_SKIN_HUE_DEG: f32 = 55.0;
const OK_SKIN_WIDTH_DEG: f32 = 26.0;
const OK_SKIN_GUARD: f32 = 0.45;

const OK_VIBRANCE_STRENGTH: f32 = 2.0;
const OK_CHROMA_REFERENCE: f32 = 0.16;

// Bands are selected from the chroma vector divided by its own length plus a
// floor. Dividing by the length alone gives hue, which is the arctangent of two
// small differences and so mostly sensor noise near neutral; selecting on it
// turned that noise into a visible cloud across the smooth gradient of a sky.
// Dividing by the raw length instead is stable but barely claims a pale colour
// at all, and a pale sky is still unambiguously blue to the photographer
// adjusting it. The floor buys both: a true direction at useful chroma, folding
// smoothly to nothing as a colour approaches grey.
//
// A lower floor claims pale colours harder and carries more of their noise; a
// lower softness makes bands more selective. At these values a pale sky takes
// around two fifths of the adjustment against a tenth for the raw vector.
const OK_BAND_CHROMA_FLOOR: f32 = 0.02;
const OK_BAND_SOFTNESS: f32 = 0.18;

// A neighbourhood average of the chroma vector carries almost none of the noise
// a single pixel does, because chroma noise is high frequency while the colour
// of a region is not. Where the two agree, the average can be normalised far
// harder than a lone pixel and a band claims a pale colour in full.
const OK_BAND_NEIGHBOUR_FLOOR: f32 = 0.012;

// Where they disagree the average is straddling an edge, and normalising it
// would drag one side's colour identity across the boundary as a halo. The
// pixel's own vector takes over there. The window is wide enough that a sky
// meeting a sea, both of them blue, still counts as agreement.
const OK_BAND_TRUST_LOW: f32 = 0.30;
const OK_BAND_TRUST_HIGH: f32 = 0.70;

// Which band a colour belongs to is decided by direction alone, at full
// selectivity whatever its chroma, and how much of the adjustment it takes is
// decided separately here. Folding the two together made the share collapse
// faster and faster as a colour approached neutral and then stop dead, which
// drew a hard edge across the smooth part of a sky passing from blue to warm.
// Kept apart, the fade through neutral has the shape of a smoothstep, easing
// out at both ends instead of falling off a cliff.
/// How far what a band has not claimed is dimmed when showing its selection.
const OK_PREVIEW_DIM: f32 = 0.30;

const OK_BAND_CONFIDENCE_LOW: f32 = 0.002;
const OK_BAND_CONFIDENCE_HIGH: f32 = 0.045;

// The share a band takes for a colour sitting exactly on its own centre.
// Because the centres are unevenly spaced, this ceiling differs per band, and
// dividing by it is what lets each own its colour outright. Left undivided,
// purple reached forty-four percent where aqua reached eighty-five, so the same
// slider did half as much work depending on which swatch was selected.
// Generated by oklab.rs, which asserts these against the band geometry.
const OK_BAND_PEAKS = array<f32, 8>(
    0.68742, 0.59133, 0.54055, 0.59582, 0.85086, 0.78555, 0.43665, 0.45839
);

/// Cube root that keeps its argument's sign. A wide working space carries
/// channels below zero, and a plain power of a negative base is not a number.
fn ok_signed_cbrt(v: vec3<f32>) -> vec3<f32> {
    return sign(v) * pow(abs(v), vec3<f32>(1.0 / 3.0));
}

fn oklab_from_working(c: vec3<f32>) -> vec3<f32> {
    return LMS_TO_OKLAB * ok_signed_cbrt(PP_TO_LMS * c);
}

fn working_from_oklab(lab: vec3<f32>) -> vec3<f32> {
    let lms = OKLAB_TO_LMS * lab;
    return LMS_TO_PP * (lms * lms * lms);
}

fn ok_hue_distance(a: f32, b: f32) -> f32 {
    let d = abs(a - b) % 360.0;
    return min(d, 360.0 - d);
}

/// Hue, saturation, vibrance and the eight-way colour panel, in one pass
/// through Oklch.
///
/// Scaling toward luminance in linear RGB, which these controls did before,
/// moves along a line in a space where hue is not preserved: raising chroma by
/// half drifts an orange about thirteen degrees and a deep blue about ten. The
/// same change in Oklch holds hue to within measurement noise, which is the
/// whole reason the conversion is worth its cost. Doing all four here also
/// spends two conversions rather than the six the separate controls cost.
fn apply_oklch_color(
    color: vec3<f32>,
    neighbourhood: vec3<f32>,
    hue_shift_degrees: f32,
    sat: f32,
    vib: f32,
    hsl: array<HslColor, 8>
) -> vec3<f32> {
    let lab = oklab_from_working(color);
    var l = lab.x;
    var chroma = length(lab.yz);
    var hue = degrees(atan2(lab.z, lab.y));
    if (hue < 0.0) {
        hue = hue + 360.0;
    }

    let neighbour_lab = oklab_from_working(neighbourhood);
    let neighbour_chroma = length(neighbour_lab.yz);

    // Relative, so it reads the same on a pale sky as on a saturated flower.
    let disagreement = length(lab.yz - neighbour_lab.yz)
        / (chroma + neighbour_chroma + OK_BAND_CHROMA_FLOOR);
    let trust = 1.0 - smoothstep(OK_BAND_TRUST_LOW, OK_BAND_TRUST_HIGH, disagreement);

    let own_direction = lab.yz / max(chroma, 1e-8);
    let neighbour_direction = neighbour_lab.yz / max(neighbour_chroma, 1e-8);
    let blended = mix(own_direction, neighbour_direction, trust);
    let selector = blended / max(length(blended), 1e-8);

    let confidence = smoothstep(
        OK_BAND_CONFIDENCE_LOW,
        OK_BAND_CONFIDENCE_HIGH,
        mix(chroma, neighbour_chroma, trust)
    );

    var weights: array<f32, 8>;
    var total: f32 = 0.0;
    for (var i = 0u; i < 8u; i = i + 1u) {
        let direction = radians(OK_BAND_CENTERS[i]);
        let alignment = selector.x * cos(direction) + selector.y * sin(direction);
        let weight = exp(alignment / OK_BAND_SOFTNESS);
        weights[i] = weight;
        total = total + weight;
    }

    var band_hue: f32 = 0.0;
    var band_sat: f32 = 0.0;
    var band_lum: f32 = 0.0;
    var shares: array<f32, 8>;
    for (var i = 0u; i < 8u; i = i + 1u) {
        // A colour no band owns divides evenly between all eight, so measuring
        // each band's share against an even one leaves neutrals untouched with
        // no threshold to cross.
        let raw = max((weights[i] / total - 0.125) / 0.875, 0.0);
        let share = min(raw / OK_BAND_PEAKS[i], 1.0) * confidence;
        shares[i] = share;
        band_hue = band_hue + hsl[i].hue * 2.0 * share;
        band_sat = band_sat + hsl[i].saturation * share;
        band_lum = band_lum + hsl[i].luminance * share;
    }

    // Showing which pixels a band has claimed, the way Capture One does: what
    // the band owns keeps its colour, everything else drains and dims, and a
    // partial claim reads as partial on both counts. Selection is otherwise
    // only visible through the artefacts it causes.
    let previewed = adjustments.global.color_mixer_preview;
    if (previewed >= 0.0) {
        let claimed = shares[u32(round(clamp(previewed, 0.0, 7.0)))];
        return working_from_oklab(vec3<f32>(
            l * mix(OK_PREVIEW_DIM, 1.0, claimed),
            lab.y * claimed,
            lab.z * claimed
        ));
    }

    hue = hue + band_hue + hue_shift_degrees;
    // Oklab lightness is perceptual, roughly the cube root of luminance, so
    // scaling it directly hits about three times harder than the luminance
    // scale this control used to be. The root restores the slider's reach.
    l = max(l * pow(max(1.0 + band_lum, 0.0), 1.0 / 3.0), 0.0);
    chroma = max(chroma * (1.0 + band_sat), 0.0);

    chroma = max(chroma * (1.0 + sat), 0.0);

    // Vibrance lifts what is not already saturated, and holds back around the
    // hue skin occupies. That restraint is a deliberate aesthetic choice, not a
    // correction for a colour space that misbehaves.
    if (vib != 0.0) {
        let headroom = 1.0 - smoothstep(0.2, 1.6, chroma / OK_CHROMA_REFERENCE);
        let from_skin = ok_hue_distance(hue, OK_SKIN_HUE_DEG) / OK_SKIN_WIDTH_DEG;
        let guard = 1.0 - OK_SKIN_GUARD * exp(-from_skin * from_skin);
        chroma = max(chroma * (1.0 + vib * OK_VIBRANCE_STRENGTH * headroom * guard), 0.0);
    }

    let radians_hue = radians(hue);
    return working_from_oklab(vec3<f32>(l, chroma * cos(radians_hue), chroma * sin(radians_hue)));
}

/// Saturation and vibrance alone, for the callers that carry no hue or band
/// adjustments of their own.
fn apply_oklch_saturation(color: vec3<f32>, sat: f32, vib: f32) -> vec3<f32> {
    let none = array<HslColor, 8>(
        HslColor(0.0, 0.0, 0.0, 0.0), HslColor(0.0, 0.0, 0.0, 0.0),
        HslColor(0.0, 0.0, 0.0, 0.0), HslColor(0.0, 0.0, 0.0, 0.0),
        HslColor(0.0, 0.0, 0.0, 0.0), HslColor(0.0, 0.0, 0.0, 0.0),
        HslColor(0.0, 0.0, 0.0, 0.0), HslColor(0.0, 0.0, 0.0, 0.0)
    );
    return apply_oklch_color(color, color, 0.0, sat, vib, none);
}

fn apply_color_grading(color: vec3<f32>, shadows: ColorGradeSettings, midtones: ColorGradeSettings, highlights: ColorGradeSettings, global: ColorGradeSettings, blending: f32, balance: f32) -> vec3<f32> {
    let luma = get_luma(max(vec3(0.0), color));
    let base_shadow_crossover = 0.1;
    let base_highlight_crossover = 0.5;
    let balance_range = 0.5;
    let shadow_crossover = base_shadow_crossover + max(0.0, -balance) * balance_range;
    let highlight_crossover = base_highlight_crossover - max(0.0, balance) * balance_range;
    let feather = 0.2 * blending;
    let final_shadow_crossover = min(shadow_crossover, highlight_crossover - 0.01);
    let shadow_mask = 1.0 - smoothstep(final_shadow_crossover - feather, final_shadow_crossover + feather, luma);
    let highlight_mask = smoothstep(highlight_crossover - feather, highlight_crossover + feather, luma);
    let midtone_mask = max(0.0, 1.0 - shadow_mask - highlight_mask);
    let global_mask = 1.0;
    var graded_color = color;
    let shadow_sat_strength = 0.1;
    let shadow_lum_strength = 0.5;
    let midtone_sat_strength = 0.6;
    let midtone_lum_strength = 0.8;
    let highlight_sat_strength = 0.8;
    let highlight_lum_strength = 1.0;
    let global_sat_strength = 1.0;
    let global_lum_strength = 1.0;
    if (shadows.saturation > 0.001) { let tint_rgb = hsv_to_rgb(vec3<f32>(shadows.hue, 1.0, 1.0)); graded_color += (tint_rgb - 0.5) * shadows.saturation * shadow_mask * shadow_sat_strength; }
    graded_color += shadows.luminance * shadow_mask * shadow_lum_strength;
    if (midtones.saturation > 0.001) { let tint_rgb = hsv_to_rgb(vec3<f32>(midtones.hue, 1.0, 1.0)); graded_color += (tint_rgb - 0.5) * midtones.saturation * midtone_mask * midtone_sat_strength; }
    graded_color += midtones.luminance * midtone_mask * midtone_lum_strength;
    if (highlights.saturation > 0.001) { let tint_rgb = hsv_to_rgb(vec3<f32>(highlights.hue, 1.0, 1.0)); graded_color += (tint_rgb - 0.5) * highlights.saturation * highlight_mask * highlight_sat_strength; }
    graded_color += highlights.luminance * highlight_mask * highlight_lum_strength;
    if (global.saturation > 0.001) { let tint_rgb = hsv_to_rgb(vec3<f32>(global.hue, 1.0, 1.0)); graded_color += (tint_rgb - 0.5) * global.saturation * global_mask * global_sat_strength; }
    graded_color += global.luminance * global_mask * global_lum_strength;
    return graded_color;
}

fn apply_local_contrast(
    processed_color_linear: vec3<f32>,
    blurred_color_input_space: vec3<f32>,
    amount: f32,
    is_raw: u32,
    mode: u32,
    threshold: f32
) -> vec3<f32> {
    if (amount == 0.0) {
        return processed_color_linear;
    }

    var blurred_color_linear: vec3<f32>;
    if (is_raw == 1u) {
        blurred_color_linear = blurred_color_input_space;
    } else {
        blurred_color_linear = input_to_working(blurred_color_input_space);
    }

    if (amount < 0.0) {
        var blur_amount = -amount;
        if (mode == 0u) {
            blur_amount = blur_amount * 0.5;
        }
        return mix(processed_color_linear, blurred_color_linear, blur_amount);
    }

    let center_luma = get_luma(processed_color_linear);

    let shadow_threshold = select(0.03, 0.1, is_raw == 1u);
    let shadow_protection = smoothstep(0.0, shadow_threshold, center_luma);
    let highlight_protection = 1.0 - smoothstep(0.9, 1.0, center_luma);
    let midtone_mask = shadow_protection * highlight_protection;

    if (midtone_mask < 0.001) {
        return processed_color_linear;
    }

    let blurred_luma = get_luma(blurred_color_linear);
    let safe_center_luma = max(center_luma, 0.0001);
    let safe_blurred_luma = max(blurred_luma, 0.0001);

    let log_ratio = log2(safe_center_luma / safe_blurred_luma);
    var effective_amount = amount;

    if (mode == 0u) {
        let edge_magnitude = abs(log_ratio);
        let normalized_edge = clamp(edge_magnitude / 3.0, 0.0, 1.0);
        let edge_dampener = 1.0 - pow(normalized_edge, 0.5);
        let edge_mask = smoothstep(threshold * 0.5, threshold * 1.5, edge_magnitude);
        effective_amount = amount * edge_dampener * edge_mask * 0.8;
    } else {
        effective_amount = amount;
    }

    let contrast_factor = exp2(log_ratio * effective_amount);
    let final_color = processed_color_linear * contrast_factor;

    return mix(processed_color_linear, final_color, midtone_mask);
}

fn sharpen_perc(c: vec3<f32>, is_raw: u32) -> f32 {
    if (is_raw == 1u) {
        return sqrt(max(get_luma(c), 0.0));
    }
    return max(get_display_luma(c), 0.0);
}

fn sharpen_tap(coords: vec2<i32>, max_idx: vec2<i32>, is_raw: u32) -> f32 {
    let c = clamp(coords, vec2<i32>(0), max_idx);
    return sharpen_perc(textureLoad(input_texture, vec2<u32>(c), 0).rgb, is_raw);
}

fn sharpen_tap_bilinear(p: vec2<f32>, max_idx: vec2<i32>, is_raw: u32) -> f32 {
    let fl = floor(p);
    let w = p - fl;
    let b = vec2<i32>(fl);
    let s00 = sharpen_tap(b, max_idx, is_raw);
    let s10 = sharpen_tap(b + vec2<i32>(1, 0), max_idx, is_raw);
    let s01 = sharpen_tap(b + vec2<i32>(0, 1), max_idx, is_raw);
    let s11 = sharpen_tap(b + vec2<i32>(1, 1), max_idx, is_raw);
    return mix(mix(s00, s10, w.x), mix(s01, s11, w.x), w.y);
}

fn sharpen_soft_limit(v: f32, lo: f32, hi: f32, margin: f32) -> f32 {
    let m = max(margin, 1e-5);
    if (v > hi) {
        let e = v - hi;
        return hi + e / (1.0 + e / m);
    }
    if (v < lo) {
        let e = lo - v;
        return lo - e / (1.0 + e / m);
    }
    return v;
}

fn apply_sharpen(
    color: vec3<f32>,
    b1_in: vec3<f32>,
    b2_in: vec3<f32>,
    coords_i: vec2<i32>,
    amount: f32,
    threshold: f32,
    is_raw: u32
) -> vec3<f32> {
    if (abs(amount) < 0.0005) {
        return color;
    }

    if (amount < 0.0) {
        var b1_lin = b1_in;
        if (is_raw == 0u) { b1_lin = input_to_working(b1_in); }
        return mix(color, b1_lin, clamp(-amount * 0.5, 0.0, 1.0));
    }

    var color_enc = color;
    if (is_raw == 0u) {
        color_enc = linear_to_srgb_extended(prophoto_to_srgb(color));
    }
    let l = select(get_display_luma(color_enc), sqrt(max(get_luma(color), 0.0)), is_raw == 1u);
    let l1 = sharpen_perc(b1_in, is_raw);
    let l2 = sharpen_perc(b2_in, is_raw);

    let d0 = l  - l1;
    let d1 = l1 - l2;

    let t = max(threshold * 0.15, 1e-4);
    let g0 = smoothstep(t * 0.20, t * 0.85, abs(d0));
    let g1 = smoothstep(t * 0.12, t * 0.55, abs(d1));

    let boost = (d0 * 1.20 * g0
               + d1 * 0.70 * g1) * amount;

    let dims = vec2<i32>(textureDimensions(input_texture));
    let max_idx = dims - vec2<i32>(1);
    let kw = array<f32, 5>(0.01853, -0.21023, 1.38348, -0.21023, 0.01853);

    var acc = 0.0;
    var center_tap = 0.0;
    var lo = 1.0e9;
    var hi = -1.0e9;
    var gx = 0.0;
    var gy = 0.0;

    for (var iy = 0; iy < 5; iy = iy + 1) {
        let oy = iy - 2;
        let ky = kw[iy];
        let cy = clamp(coords_i.y + oy, 0, max_idx.y);
        for (var ix = 0; ix < 5; ix = ix + 1) {
            let ox = ix - 2;
            let cx = clamp(coords_i.x + ox, 0, max_idx.x);
            let sl = sharpen_perc(textureLoad(input_texture, vec2<u32>(vec2<i32>(cx, cy)), 0).rgb, is_raw);

            acc += sl * kw[ix] * ky;
            lo = min(lo, sl);
            hi = max(hi, sl);

            if (ox == 0 && oy == 0) { center_tap = sl; }

            if (abs(ox) <= 1 && abs(oy) <= 1) {
                gx += sl * f32(ox) * (2.0 - abs(f32(oy)));
                gy += sl * f32(oy) * (2.0 - abs(f32(ox)));
            }
        }
    }

    let deconv_delta = (acc - center_tap) * clamp(amount * 0.60, 0.0, 1.0);

    var l_new = l + boost + deconv_delta;

    let range = max(hi - lo, 1e-5);
    l_new = sharpen_soft_limit(
        l_new,
        lo - range * 0.06,
        hi + range * 0.10,
        range * 0.12
    );

    let g2 = gx * gx + gy * gy;
    if (g2 > 1e-6) {
        let diagonality = clamp(2.0 * abs(gx * gy) / g2, 0.0, 1.0);
        let edge_present = smoothstep(t * 0.5, t * 2.5, sqrt(g2) * 0.25);
        let aa = 0.90 * diagonality * edge_present;

        if (aa > 0.002) {
            let inv_g = inverseSqrt(g2);
            let tang = vec2<f32>(-gy, gx) * inv_g * 1.30;
            let p = vec2<f32>(coords_i) + vec2<f32>(0.5);

            let tp = sharpen_tap_bilinear(p + tang - vec2<f32>(0.5), max_idx, is_raw);
            let tn = sharpen_tap_bilinear(p - tang - vec2<f32>(0.5), max_idx, is_raw);
            let l_tan = (tp + tn + l * 2.0) * 0.25;

            let l_aa = l_tan + (l_new - l);
            l_new = mix(l_new, l_aa, aa);
        }
    }

    let shadow_floor = select(0.03, 0.10, is_raw == 1u);
    let prot = smoothstep(0.0, shadow_floor, l) * (1.0 - smoothstep(0.92, 1.0, l));
    l_new = max(mix(l, l_new, prot), 0.0);

    let ratio = l_new / max(l, 1e-4);
    if (is_raw == 1u) {
        return color * (ratio * ratio);
    }
    return input_to_working(max(color_enc * ratio, vec3<f32>(0.0)));
}

fn apply_centre_local_contrast(
    color_in: vec3<f32>,
    centre_amount: f32,
    coords_i: vec2<i32>,
    blurred_color_srgb: vec3<f32>,
    is_raw: u32
) -> vec3<f32> {
    if (centre_amount == 0.0) {
        return color_in;
    }
    let full_dims_f = vec2<f32>(textureDimensions(input_texture));
    let coord_f = vec2<f32>(coords_i);
    let midpoint = 0.4;
    let feather = 0.375;
    let aspect = full_dims_f.y / full_dims_f.x;
    let uv_centered = (coord_f / full_dims_f - 0.5) * 2.0;
    let d = length(uv_centered * vec2<f32>(1.0, aspect)) * 0.5;
    let vignette_mask = smoothstep(midpoint - feather, midpoint + feather, d);
    let centre_mask = 1.0 - vignette_mask;

    const CLARITY_SCALE: f32 = 0.9;
    var processed_color = color_in;
    let clarity_strength = centre_amount * (2.0 * centre_mask - 1.0) * CLARITY_SCALE;

    if (abs(clarity_strength) > 0.001) {
        processed_color = apply_local_contrast(processed_color, blurred_color_srgb, clarity_strength, is_raw, 1u, 0.0);
    }

    return processed_color;
}

fn apply_centre_tonal_and_color(
    color_in: vec3<f32>,
    centre_amount: f32,
    coords_i: vec2<i32>
) -> vec3<f32> {
    if (centre_amount == 0.0) {
        return color_in;
    }
    let full_dims_f = vec2<f32>(textureDimensions(input_texture));
    let coord_f = vec2<f32>(coords_i);
    let midpoint = 0.4;
    let feather = 0.375;
    let aspect = full_dims_f.y / full_dims_f.x;
    let uv_centered = (coord_f / full_dims_f - 0.5) * 2.0;
    let d = length(uv_centered * vec2<f32>(1.0, aspect)) * 0.5;
    let vignette_mask = smoothstep(midpoint - feather, midpoint + feather, d);
    let centre_mask = 1.0 - vignette_mask;

    const EXPOSURE_SCALE: f32 = 0.5;
    const VIBRANCE_SCALE: f32 = 0.4;
    const SATURATION_CENTER_SCALE: f32 = 0.3;
    const SATURATION_EDGE_SCALE: f32 = 0.8;

    var processed_color = color_in;

    let exposure_boost = centre_mask * centre_amount * EXPOSURE_SCALE;
    processed_color = apply_filmic_exposure(processed_color, exposure_boost);

    let vibrance_center_boost = centre_mask * centre_amount * VIBRANCE_SCALE;
    let saturation_center_boost = centre_mask * centre_amount * SATURATION_CENTER_SCALE;
    let saturation_edge_effect = -(1.0 - centre_mask) * centre_amount * SATURATION_EDGE_SCALE;
    let total_saturation_effect = saturation_center_boost + saturation_edge_effect;
    processed_color = apply_oklch_saturation(processed_color, total_saturation_effect, vibrance_center_boost);

    return processed_color;
}

fn apply_dehaze(color: vec3<f32>, blurred_color_input_space: vec3<f32>, is_raw: u32, amount: f32) -> vec3<f32> {
    if (amount == 0.0) { return color; }

    var blurred_linear: vec3<f32>;
    if (is_raw == 1u) {
        blurred_linear = blurred_color_input_space;
    } else {
        blurred_linear = input_to_working(blurred_color_input_space);
    }

    let atmospheric_light = vec3<f32>(0.95, 0.97, 1.0);

    if (amount > 0.0) {
        let pixel_dark = min(color.r, min(color.g, color.b));
        let regional_dark = min(blurred_linear.r, min(blurred_linear.g, blurred_linear.b));
        let pixel_luma = get_luma(max(color, vec3<f32>(0.0)));
        let blurred_luma = get_luma(max(blurred_linear, vec3<f32>(0.0)));
        let edge_diff = abs(pow(pixel_luma, 0.5) - pow(blurred_luma, 0.5));
        let halo_protection = smoothstep(0.02, 0.15, edge_diff);
        let spatial_dark = mix(regional_dark, pixel_dark, halo_protection);
        let safe_dark = max(spatial_dark - 0.02, 0.0);
        let mapped_haze = safe_dark / (safe_dark + 0.2);
        let t = max(1.0 - amount * mapped_haze * 0.85, 0.15);
        var recovered = (color - atmospheric_light) / t + atmospheric_light;
        let rec_luma = get_luma(max(recovered, vec3<f32>(0.0)));
        let shadow_lift = smoothstep(0.1, 0.0, rec_luma) * (1.0 - t) * 0.15;
        recovered += shadow_lift;
        let haze_removed = 1.0 - t;
        let sat_boost = haze_removed * 0.5;
        let final_luma = get_luma(max(recovered, vec3<f32>(0.0)));
        recovered = mix(vec3<f32>(final_luma), recovered, 1.0 + sat_boost);
        return max(recovered, vec3<f32>(0.0));
    } else {
        let regional_dark = min(blurred_linear.r, min(blurred_linear.g, blurred_linear.b));
        let safe_dark = max(regional_dark - 0.02, 0.0);
        let mapped_depth = safe_dark / (safe_dark + 0.2);
        let depth_factor = mix(0.4, 1.0, mapped_depth);
        return mix(color, atmospheric_light, abs(amount) * 0.7 * depth_factor);
    }
}

fn apply_noise_reduction(
    center_linear: vec3<f32>,
    coords_i: vec2<i32>,
    luma_amount: f32,
    color_amount: f32,
    scale: f32,
    is_raw: u32
) -> vec3<f32> {
    let luma_a  = clamp(luma_amount,  0.0, 1.0);
    let color_a = clamp(color_amount, 0.0, 1.0);
    if (luma_a < 0.001 && color_a < 0.001) {
        return center_linear;
    }

    let dims = vec2<i32>(textureDimensions(input_texture));
    let max_idx = dims - vec2<i32>(1);
    let center_safe   = max(center_linear, vec3<f32>(0.0));
    let center_luma   = get_luma(center_safe);
    let center_chroma = center_linear - vec3<f32>(center_luma);

    let res_factor = clamp(sqrt(scale), 0.5, 2.0);

    var new_luma   = center_luma;
    var new_chroma = center_chroma;

    // --- LUMA NOISE REDUCTION ---
    if (luma_a > 0.001) {
        let l_curve = sqrt(luma_a);

        let stride_f = mix(1.0, 2.0, smoothstep(0.45, 0.95, luma_a)) * res_factor;
        let extra    = clamp(stride_f - 1.0, 0.0, 1.0);

        let l_spatial = mix(1.0, 1.5, l_curve);
        let l_spat_n  = -1.0 / max(2.0 * l_spatial * l_spatial, 1e-6);

        let h1 = hash(vec2<f32>(coords_i));
        let h2 = hash(vec2<f32>(coords_i) + vec2<f32>(17.31, 71.13));

        var samp_luma: array<f32, 25>;
        var samp_spat: array<f32, 25>;
        var lmin: f32 = center_luma;
        var lmax: f32 = center_luma;

        samp_luma[0] = center_luma;
        samp_spat[0] = 1.0;

        var idx: u32 = 1u;
        for (var dy: i32 = -2; dy <= 2; dy = dy + 1) {
            for (var dx: i32 = -2; dx <= 2; dx = dx + 1) {
                if (dx == 0 && dy == 0) { continue; }

                let ring = max(abs(dx), abs(dy));
                let ring_factor = select(0.5, 1.0, ring == 2);
                let grow = 1.0 + extra * ring_factor;

                let jx = (h1 - 0.5) * 2.0 * extra;
                let jy = (h2 - 0.5) * 2.0 * extra;

                let off_f = vec2<f32>(f32(dx) * grow + jx, f32(dy) * grow + jy);
                let off   = vec2<i32>(i32(round(off_f.x)), i32(round(off_f.y)));
                let coord = clamp(coords_i + off, vec2<i32>(0), max_idx);

                var s = textureLoad(input_texture, vec2<u32>(coord), 0).rgb;
                if (is_raw == 0u) { s = input_to_working(s); }
                let s_luma = get_luma(max(s, vec3<f32>(0.0)));
                samp_luma[idx] = s_luma;
                samp_spat[idx] = exp(f32(dx * dx + dy * dy) * l_spat_n);
                lmin = min(lmin, s_luma);
                lmax = max(lmax, s_luma);
                idx = idx + 1u;
            }
        }

        let luma_range    = lmax - lmin;
        let edge_strength = smoothstep(0.04, 0.20, luma_range);
        let edge_midpoint = (lmin + lmax) * 0.5;
        let center_side   = center_luma > edge_midpoint;

        let l_range_tol = mix(
            mix(0.025, 0.075, l_curve),
            mix(0.010, 0.025, l_curve),
            edge_strength
        );

        var samp_gate: array<f32, 25>;
        var sum_a: f32 = 0.0;
        var w_a:   f32 = 0.0;
        for (var k: u32 = 0u; k < 25u; k = k + 1u) {
            let diff = abs(samp_luma[k] - center_luma);
            let g_range = 1.0 - smoothstep(l_range_tol * 0.6, l_range_tol, diff);
            let s_side  = samp_luma[k] > edge_midpoint;
            let g_side  = select(0.0, 1.0, s_side == center_side);
            let g_edge  = mix(1.0, g_side, edge_strength);
            let w = samp_spat[k] * g_range * g_edge;
            samp_gate[k] = w;
            sum_a += samp_luma[k] * w;
            w_a   += w;
        }
        let initial_mean = sum_a / max(w_a, 1e-4);

        let outlier_tol = mix(0.07, 0.025, edge_strength);
        var sum_b: f32 = 0.0;
        var w_b:   f32 = 0.0;
        for (var k: u32 = 0u; k < 25u; k = k + 1u) {
            let init_w = samp_gate[k];
            if (init_w > 0.0001) {
                let d = samp_luma[k] - initial_mean;
                let r = abs(d) / outlier_tol;
                let bisq = max(0.0, 1.0 - r * r);
                let outlier_w = bisq * bisq;
                let w = init_w * outlier_w;
                sum_b += samp_luma[k] * w;
                w_b   += w;
            }
        }
        let robust_luma = select(initial_mean, sum_b / max(w_b, 1e-6), w_b > 0.01);

        let strength = luma_a * mix(1.0, 0.6, edge_strength);
        new_luma = mix(center_luma, robust_luma, strength);
    }

    if (color_a > 0.001) {
        let center_r_y = center_linear.r - center_luma;
        let center_b_y = center_linear.b - center_luma;
        let c_curve = sqrt(color_a);
        let stride_f = mix(2.0, 3.5, c_curve) * res_factor;

        let c_spatial = mix(2.0, 3.5, c_curve);
        let c_spat_n  = -1.0 / max(2.0 * c_spatial * c_spatial, 1e-6);

        let luma_tol = mix(0.12, 0.04, c_curve);
        let luma_n   = -1.0 / max(2.0 * luma_tol * luma_tol, 1e-6);

        let chroma_tol = mix(0.20, 0.08, c_curve);
        let chroma_n   = -1.0 / max(2.0 * chroma_tol * chroma_tol, 1e-6);

        let jh1 = hash(vec2<f32>(coords_i) + vec2<f32>(43.7, 91.1));
        let jh2 = hash(vec2<f32>(coords_i) + vec2<f32>(73.3, 17.9));
        let jx  = (jh1 - 0.5) * stride_f * 0.5;
        let jy  = (jh2 - 0.5) * stride_f * 0.5;

        var sum_r: f32 = center_r_y;
        var sum_b: f32 = center_b_y;
        var w_sum: f32 = 1.0;

        for (var dy: i32 = -2; dy <= 2; dy = dy + 1) {
            for (var dx: i32 = -2; dx <= 2; dx = dx + 1) {
                if (dx == 0 && dy == 0) { continue; }
                let off_f = vec2<f32>(f32(dx) * stride_f + jx, f32(dy) * stride_f + jy);
                let off   = vec2<i32>(i32(round(off_f.x)), i32(round(off_f.y)));
                let coord = clamp(coords_i + off, vec2<i32>(0), max_idx);
                var s = textureLoad(input_texture, vec2<u32>(coord), 0).rgb;

                if (is_raw == 0u) { s = input_to_working(s); }

                let s_safe = max(s, vec3<f32>(0.0));
                let s_luma = get_luma(s_safe);
                let s_r_y  = s.r - s_luma;
                let s_b_y  = s.b - s_luma;

                let r2  = f32(dx * dx + dy * dy);
                let w_s = exp(r2 * c_spat_n);
                let dl  = s_luma - center_luma;
                let w_l = exp(dl * dl * luma_n);
                let dr  = s_r_y - center_r_y;
                let db  = s_b_y - center_b_y;
                let dc2 = dr * dr + db * db;
                let w_c = exp(dc2 * chroma_n);
                let w = w_s * w_l * w_c;

                sum_r += s_r_y * w;
                sum_b += s_b_y * w;
                w_sum += w;
            }
        }
        let filtered_r_y = sum_r / max(w_sum, 1e-6);
        let filtered_b_y = sum_b / max(w_sum, 1e-6);

        let new_r_y = mix(center_r_y, filtered_r_y, color_a);
        let new_b_y = mix(center_b_y, filtered_b_y, color_a);
        let new_g_y = -(LUMA_COEFF.r * new_r_y + LUMA_COEFF.b * new_b_y) / LUMA_COEFF.g;

        new_chroma = vec3<f32>(new_r_y, new_g_y, new_b_y);
    }

    return vec3<f32>(new_luma) + new_chroma;
}

fn apply_ca_correction(coords: vec2<u32>, ca_rc: f32, ca_by: f32) -> vec3<f32> {
    let dims = vec2<f32>(textureDimensions(input_texture));
    let center = dims / 2.0;
    let current_pos = vec2<f32>(coords);

    let to_center = current_pos - center;
    let dist = length(to_center);

    if (dist == 0.0) {
        return textureLoad(input_texture, coords, 0).rgb;
    }

    let dir = to_center / dist;

    let red_shift = dir * dist * ca_rc;
    let blue_shift = dir * dist * ca_by;

    let red_coords = vec2<i32>(round(current_pos - red_shift));
    let blue_coords = vec2<i32>(round(current_pos - blue_shift));
    let green_coords = vec2<i32>(current_pos);

    let max_coords = vec2<i32>(dims - 1.0);

    let r = textureLoad(input_texture, vec2<u32>(clamp(red_coords, vec2<i32>(0), max_coords)), 0).r;
    let g = textureLoad(input_texture, vec2<u32>(clamp(green_coords, vec2<i32>(0), max_coords)), 0).g;
    let b = textureLoad(input_texture, vec2<u32>(clamp(blue_coords, vec2<i32>(0), max_coords)), 0).b;

    return vec3<f32>(r, g, b);
}

const AGX_EPSILON: f32 = 1.0e-6;
const AGX_MIN_EV: f32 = -15.2;
const AGX_MAX_EV: f32 = 5.0;
const AGX_RANGE_EV: f32 = AGX_MAX_EV - AGX_MIN_EV;
const AGX_GAMMA: f32 = 2.4;
const AGX_SLOPE: f32 = 2.3843;
const AGX_TOE_POWER: f32 = 1.5;
const AGX_SHOULDER_POWER: f32 = 1.5;
const AGX_TOE_TRANSITION_X: f32 = 0.6060606;
const AGX_TOE_TRANSITION_Y: f32 = 0.43446;
const AGX_SHOULDER_TRANSITION_X: f32 = 0.6060606;
const AGX_SHOULDER_TRANSITION_Y: f32 = 0.43446;
const AGX_INTERCEPT: f32 = -1.0112;
const AGX_TOE_SCALE: f32 = -1.0359;
const AGX_SHOULDER_SCALE: f32 = 1.3475;
const AGX_TARGET_BLACK_PRE_GAMMA: f32 = 0.0;
const AGX_TARGET_WHITE_PRE_GAMMA: f32 = 1.0;

fn agx_sigmoid(x: f32, power: f32) -> f32 {
    return x / pow(1.0 + pow(x, power), 1.0 / power);
}

fn agx_scaled_sigmoid(x: f32, scale: f32, slope: f32, power: f32, transition_x: f32, transition_y: f32) -> f32 {
    return scale * agx_sigmoid(slope * (x - transition_x) / scale, power) + transition_y;
}

fn agx_apply_curve_channel(x: f32) -> f32 {
    var result: f32 = 0.0;
    if (x < AGX_TOE_TRANSITION_X) {
        result = agx_scaled_sigmoid(x, AGX_TOE_SCALE, AGX_SLOPE, AGX_TOE_POWER, AGX_TOE_TRANSITION_X, AGX_TOE_TRANSITION_Y);
    } else if (x <= AGX_SHOULDER_TRANSITION_X) {
        result = AGX_SLOPE * x + AGX_INTERCEPT;
    } else {
        result = agx_scaled_sigmoid(x, AGX_SHOULDER_SCALE, AGX_SLOPE, AGX_SHOULDER_POWER, AGX_SHOULDER_TRANSITION_X, AGX_SHOULDER_TRANSITION_Y);
    }
    return clamp(result, AGX_TARGET_BLACK_PRE_GAMMA, AGX_TARGET_WHITE_PRE_GAMMA);
}

fn agx_compress_gamut(c: vec3<f32>) -> vec3<f32> {
    let min_c = min(c.r, min(c.g, c.b));
    if (min_c < 0.0) {
        return c - min_c;
    }
    return c;
}

fn agx_tonemap(c: vec3<f32>) -> vec3<f32> {
    let x_relative = max(c / 0.18, vec3<f32>(AGX_EPSILON));
    let log_encoded = (log2(x_relative) - AGX_MIN_EV) / AGX_RANGE_EV;
    let mapped = clamp(log_encoded, vec3<f32>(0.0), vec3<f32>(1.0));

    var curved: vec3<f32>;
    curved.r = agx_apply_curve_channel(mapped.r);
    curved.g = agx_apply_curve_channel(mapped.g);
    curved.b = agx_apply_curve_channel(mapped.b);

    let final_color = pow(max(curved, vec3<f32>(0.0)), vec3<f32>(AGX_GAMMA));

    return final_color;
}

fn agx_full_transform(color_in: vec3<f32>) -> vec3<f32> {
    let compressed_color = agx_compress_gamut(color_in);
    let color_in_agx_space = adjustments.global.agx_pipe_to_rendering_matrix * compressed_color;
    let tonemapped_agx = agx_tonemap(color_in_agx_space);
    let final_color = adjustments.global.agx_rendering_to_pipe_matrix * tonemapped_agx;
    return final_color;
}

fn legacy_tonemap(c: vec3<f32>) -> vec3<f32> {
    const a: f32 = 2.51;
    const b: f32 = 0.03;
    const c_const: f32 = 2.43;
    const d: f32 = 0.59;
    const e: f32 = 0.14;

    let x = max(c, vec3<f32>(0.0));

    let numerator = x * (a * x + b);
    let denominator = x * (c_const * x + d) + e;

    let tonemapped = select(vec3<f32>(0.0), numerator / denominator, denominator > vec3<f32>(0.00001));

    return clamp(tonemapped, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn no_tonemap(c: vec3<f32>) -> vec3<f32> {
    return c;
}

fn is_default_curve(points: array<Point, 16>, count: u32) -> bool {
    if (count < 2u) {
        return false;
    }

    var is_identity = true;
    for (var i = 0u; i < count; i = i + 1u) {
        if (abs(points[i].x - points[i].y) > 0.5) {
            is_identity = false;
            break;
        }
    }

    let p0 = points[0];
    let p_last = points[count - 1u];
    let p0_is_origin = abs(p0.x - 0.0) < 0.1 && abs(p0.y - 0.0) < 0.1;
    let p_last_is_end = abs(p_last.x - 255.0) < 0.1 && abs(p_last.y - 255.0) < 0.1;

    return is_identity && p0_is_origin && p_last_is_end;
}

fn apply_all_curves(color: vec3<f32>, luma_curve: array<Point, 16>, luma_curve_count: u32, red_curve: array<Point, 16>, red_curve_count: u32, green_curve: array<Point, 16>, green_curve_count: u32, blue_curve: array<Point, 16>, blue_curve_count: u32) -> vec3<f32> {
    let red_is_default = is_default_curve(red_curve, red_curve_count);
    let green_is_default = is_default_curve(green_curve, green_curve_count);
    let blue_is_default = is_default_curve(blue_curve, blue_curve_count);
    let rgb_curves_are_active = !red_is_default || !green_is_default || !blue_is_default;

    if (rgb_curves_are_active) {
        let color_graded = vec3<f32>(
            apply_curve(color.r, red_curve, red_curve_count),
            apply_curve(color.g, green_curve, green_curve_count),
            apply_curve(color.b, blue_curve, blue_curve_count)
        );
        let luma_initial = get_display_luma(color);
        let luma_target = apply_curve(luma_initial, luma_curve, luma_curve_count);
        let luma_graded = get_display_luma(color_graded);

        let d = luma_target - luma_graded;
        var final_color = color_graded + vec3<f32>(d);

        let c_min = min(final_color.r, min(final_color.g, final_color.b));
        if (c_min < 0.0) {
            final_color = vec3<f32>(luma_target) + ((final_color - vec3<f32>(luma_target)) * luma_target) / max(luma_target - c_min, 1e-6);
        }

        let c_max = max(final_color.r, max(final_color.g, final_color.b));
        if (c_max > 1.0) {
            final_color = vec3<f32>(luma_target) + ((final_color - vec3<f32>(luma_target)) * (1.0 - luma_target)) / max(c_max - luma_target, 1e-6);
        }

        return clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0));
    } else {
        return vec3<f32>(apply_curve(color.r, luma_curve, luma_curve_count), apply_curve(color.g, luma_curve, luma_curve_count), apply_curve(color.b, luma_curve, luma_curve_count));
    }
}

fn get_mask_influence(mask_index: u32, coords: vec2<u32>) -> f32 {
    return textureLoad(mask_textures, vec2<i32>(coords), i32(mask_index), 0).r;
}

fn sample_lut_tetrahedral(uv: vec3<f32>) -> vec3<f32> {
    let dims = vec3<f32>(textureDimensions(lut_texture));
    let size = dims - vec3<f32>(1.0);
    let scaled = clamp(uv, vec3<f32>(0.0), vec3<f32>(1.0)) * size;
    let i_base = floor(scaled);
    let f = scaled - i_base;
    let coord0 = vec3<i32>(i_base);
    let coord1 = min(coord0 + vec3<i32>(1), vec3<i32>(dims) - vec3<i32>(1));
    let c000 = textureLoad(lut_texture, coord0, 0).rgb;
    let c111 = textureLoad(lut_texture, coord1, 0).rgb;

    var res = vec3<f32>(0.0);

    if (f.r > f.g) {
        if (f.g > f.b) {
            let c100 = textureLoad(lut_texture, vec3<i32>(coord1.x, coord0.y, coord0.z), 0).rgb;
            let c110 = textureLoad(lut_texture, vec3<i32>(coord1.x, coord1.y, coord0.z), 0).rgb;

            res = c000 * (1.0 - f.r) +
                  c100 * (f.r - f.g) +
                  c110 * (f.g - f.b) +
                  c111 * (f.b);
        } else if (f.r > f.b) {
            let c100 = textureLoad(lut_texture, vec3<i32>(coord1.x, coord0.y, coord0.z), 0).rgb;
            let c101 = textureLoad(lut_texture, vec3<i32>(coord1.x, coord0.y, coord1.z), 0).rgb;

            res = c000 * (1.0 - f.r) +
                  c100 * (f.r - f.b) +
                  c101 * (f.b - f.g) +
                  c111 * (f.g);
        } else {
            let c001 = textureLoad(lut_texture, vec3<i32>(coord0.x, coord0.y, coord1.z), 0).rgb;
            let c101 = textureLoad(lut_texture, vec3<i32>(coord1.x, coord0.y, coord1.z), 0).rgb;

            res = c000 * (1.0 - f.b) +
                  c001 * (f.b - f.r) +
                  c101 * (f.r - f.g) +
                  c111 * (f.g);
        }
    } else {
        if (f.b > f.g) {
            let c001 = textureLoad(lut_texture, vec3<i32>(coord0.x, coord0.y, coord1.z), 0).rgb;
            let c011 = textureLoad(lut_texture, vec3<i32>(coord0.x, coord1.y, coord1.z), 0).rgb;

            res = c000 * (1.0 - f.b) +
                  c001 * (f.b - f.g) +
                  c011 * (f.g - f.r) +
                  c111 * (f.r);
        } else if (f.b > f.r) {
            let c010 = textureLoad(lut_texture, vec3<i32>(coord0.x, coord1.y, coord0.z), 0).rgb;
            let c011 = textureLoad(lut_texture, vec3<i32>(coord0.x, coord1.y, coord1.z), 0).rgb;

            res = c000 * (1.0 - f.g) +
                  c010 * (f.g - f.b) +
                  c011 * (f.b - f.r) +
                  c111 * (f.r);
        } else {
            let c010 = textureLoad(lut_texture, vec3<i32>(coord0.x, coord1.y, coord0.z), 0).rgb;
            let c110 = textureLoad(lut_texture, vec3<i32>(coord1.x, coord1.y, coord0.z), 0).rgb;

            res = c000 * (1.0 - f.g) +
                  c010 * (f.g - f.r) +
                  c110 * (f.r - f.b) +
                  c111 * (f.b);
        }
    }

    return res;
}

fn apply_glow_bloom(
    color: vec3<f32>,
    blurred_color_input_space: vec3<f32>,
    amount: f32,
    is_raw: u32,
    exp: f32, bright: f32, con: f32, wh: f32
) -> vec3<f32> {
    if (amount <= 0.0) {
        return color;
    }

    var blurred_linear: vec3<f32>;
    if (is_raw == 1u) {
        blurred_linear = blurred_color_input_space;
    } else {
        blurred_linear = input_to_working(blurred_color_input_space);
    }

    blurred_linear = apply_linear_exposure(blurred_linear, exp);
    blurred_linear = apply_filmic_exposure(blurred_linear, bright);
    blurred_linear = apply_tonal_adjustments(blurred_linear, blurred_color_input_space, is_raw, 0.0, 0.0, wh, 0.0);

    let linear_luma = get_luma(max(blurred_linear, vec3<f32>(0.0)));

    var perceptual_luma: f32;
    if (linear_luma <= 1.0) {
        perceptual_luma = pow(max(linear_luma, 0.0), 1.0 / 2.2);
    } else {
        perceptual_luma = 1.0 + pow(linear_luma - 1.0, 1.0 / 2.2);
    }

    let luma_cutoff = mix(0.75, 0.08, clamp(amount, 0.0, 1.0));

    let cutoff_fade = smoothstep(
        luma_cutoff,
        luma_cutoff + 0.15,
        perceptual_luma
    );

    let excess = max(perceptual_luma - luma_cutoff, 0.0);

    let falloff_range = 5.5;
    let normalized = excess / falloff_range;

    let bloom_intensity =
        pow(smoothstep(0.0, 1.0, normalized), 0.45);

    var bloom_color: vec3<f32>;
    if (linear_luma > 0.01) {
        let color_ratio = blurred_linear / linear_luma;
        let warm_tint = vec3<f32>(1.03, 1.0, 0.97);
        bloom_color = color_ratio * warm_tint;
    } else {
        bloom_color = vec3<f32>(1.0, 0.99, 0.98);
    }

    let luma_factor = pow(linear_luma, 0.6);

    let black_gate_width = 0.5;
    let black_gate_raw = smoothstep(0.0, black_gate_width, linear_luma);
    let black_gate = pow(black_gate_raw, 0.5);

    bloom_color *= bloom_intensity * luma_factor * cutoff_fade * black_gate;

    let current_luma = get_luma(max(color, vec3<f32>(0.0)));
    let protection = 1.0 - smoothstep(1.0, 2.2, current_luma);

    return color + bloom_color * amount * 3.8 * protection;
}

fn apply_halation(
    color: vec3<f32>,
    blurred_color_input_space: vec3<f32>,
    amount: f32,
    is_raw: u32,
    exp: f32, bright: f32, con: f32, wh: f32
) -> vec3<f32> {
    if (amount <= 0.0) { return color; }

    var blurred_linear: vec3<f32>;
    if (is_raw == 1u) {
        blurred_linear = blurred_color_input_space;
    } else {
        blurred_linear = input_to_working(blurred_color_input_space);
    }

    blurred_linear = apply_linear_exposure(blurred_linear, exp);
    blurred_linear = apply_filmic_exposure(blurred_linear, bright);
    blurred_linear = apply_tonal_adjustments(blurred_linear, blurred_color_input_space, is_raw, 0.0, 0.0, wh, 0.0);

    let linear_luma = get_luma(max(blurred_linear, vec3<f32>(0.0)));

    var perceptual_luma: f32;
    if (linear_luma <= 1.0) {
        perceptual_luma = pow(max(linear_luma, 0.0), 1.0 / 2.2);
    } else {
        perceptual_luma = 1.0 + pow(linear_luma - 1.0, 1.0 / 2.2);
    }

    let luma_cutoff = mix(0.85, 0.1, clamp(amount, 0.0, 1.0));

    if (perceptual_luma <= luma_cutoff) { return color; }

    let excess = perceptual_luma - luma_cutoff;
    let range = max(1.5 - luma_cutoff, 0.1);
    let halation_mask = smoothstep(0.0, range * 0.6, excess);

    let halation_core = vec3<f32>(1.0, 0.15, 0.03);
    let halation_fringe = vec3<f32>(1.0, 0.32, 0.10);

    let intensity_blend = smoothstep(0.0, 0.7, halation_mask);
    let halation_tint = mix(halation_fringe, halation_core, intensity_blend);

    let glow_intensity = halation_mask * linear_luma;
    let halation_glow = halation_tint * glow_intensity;

    let color_luma = get_luma(max(color, vec3<f32>(0.0)));
    let desat_strength = halation_mask * 0.12;
    let affected_color = mix(color, vec3<f32>(color_luma), desat_strength);

    let contrast_reduced = mix(vec3<f32>(0.5), affected_color, 1.0 - halation_mask * 0.06);

    return contrast_reduced + halation_glow * amount * 2.5;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let out_dims = vec2<u32>(textureDimensions(output_texture));
    if (id.x >= out_dims.x || id.y >= out_dims.y) { return; }

    const REFERENCE_DIMENSION: f32 = 1080.0;
    let full_dims = vec2<f32>(textureDimensions(input_texture));
    let current_ref_dim = min(full_dims.x, full_dims.y);
    let scale = max(0.1, current_ref_dim / REFERENCE_DIMENSION);

    let absolute_coord = id.xy + vec2<u32>(adjustments.tile_offset_x, adjustments.tile_offset_y);
    let absolute_coord_i = vec2<i32>(absolute_coord);

    let ca_rc = adjustments.global.chromatic_aberration_red_cyan;
    let ca_by = adjustments.global.chromatic_aberration_blue_yellow;
    var color_from_texture = textureLoad(input_texture, absolute_coord, 0).rgb;
    if (abs(ca_rc) > 0.000001 || abs(ca_by) > 0.000001) {
        color_from_texture = apply_ca_correction(absolute_coord, ca_rc, ca_by);
    }
    let original_alpha = textureLoad(input_texture, absolute_coord, 0).a;

    var initial_linear_rgb: vec3<f32>;
    let is_raw = adjustments.global.is_raw_image;
    if (is_raw == 0u) {
        initial_linear_rgb = srgb_to_prophoto(srgb_to_linear(color_from_texture));
    } else {
        initial_linear_rgb = color_from_texture;
    }

    var t_exposure = adjustments.global.exposure;
    var t_brightness = adjustments.global.brightness;
    var t_contrast = adjustments.global.contrast;
    var t_highlights = adjustments.global.highlights;
    var t_shadows = adjustments.global.shadows;
    var t_whites = adjustments.global.whites;
    var t_blacks = adjustments.global.blacks;
    var t_saturation = adjustments.global.saturation;
    var t_temperature = adjustments.global.temperature;
    var t_tint = adjustments.global.tint;
    var t_vibrance = adjustments.global.vibrance;
    var t_luma_nr = adjustments.global.luma_noise_reduction;
    var t_color_nr = adjustments.global.color_noise_reduction;
    var t_clarity = adjustments.global.clarity;
    var t_dehaze = adjustments.global.dehaze;
    var t_structure = adjustments.global.structure;
    var t_glow = adjustments.global.glow_amount;
    var t_halation = adjustments.global.halation_amount;
    var t_flare = adjustments.global.flare_amount;
    var t_sharpness = adjustments.global.sharpness;
    var t_sharp_thresh = adjustments.global.sharpness_threshold;
    var t_hue = adjustments.global.hue;

    var h0_h = adjustments.global.hsl[0].hue; var h0_s = adjustments.global.hsl[0].saturation; var h0_l = adjustments.global.hsl[0].luminance;
    var h1_h = adjustments.global.hsl[1].hue; var h1_s = adjustments.global.hsl[1].saturation; var h1_l = adjustments.global.hsl[1].luminance;
    var h2_h = adjustments.global.hsl[2].hue; var h2_s = adjustments.global.hsl[2].saturation; var h2_l = adjustments.global.hsl[2].luminance;
    var h3_h = adjustments.global.hsl[3].hue; var h3_s = adjustments.global.hsl[3].saturation; var h3_l = adjustments.global.hsl[3].luminance;
    var h4_h = adjustments.global.hsl[4].hue; var h4_s = adjustments.global.hsl[4].saturation; var h4_l = adjustments.global.hsl[4].luminance;
    var h5_h = adjustments.global.hsl[5].hue; var h5_s = adjustments.global.hsl[5].saturation; var h5_l = adjustments.global.hsl[5].luminance;
    var h6_h = adjustments.global.hsl[6].hue; var h6_s = adjustments.global.hsl[6].saturation; var h6_l = adjustments.global.hsl[6].luminance;
    var h7_h = adjustments.global.hsl[7].hue; var h7_s = adjustments.global.hsl[7].saturation; var h7_l = adjustments.global.hsl[7].luminance;

    for (var i = 0u; i < adjustments.mask_count; i = i + 1u) {
        let influence = get_mask_influence(i, absolute_coord);
        if (influence > 0.001) {
            let m = adjustments.mask_adjustments[i];

            t_exposure += m.exposure * influence;
            t_brightness += m.brightness * influence;
            t_contrast += m.contrast * influence;
            t_highlights += m.highlights * influence;
            t_shadows += m.shadows * influence;
            t_whites += m.whites * influence;
            t_blacks += m.blacks * influence;

            t_saturation += m.saturation * influence;
            t_temperature += m.temperature * influence;
            t_tint += m.tint * influence;
            t_vibrance += m.vibrance * influence;

            t_luma_nr += m.luma_noise_reduction * influence;
            t_color_nr += m.color_noise_reduction * influence;
            t_clarity += m.clarity * influence;
            t_dehaze += m.dehaze * influence;
            t_structure += m.structure * influence;

            t_glow += m.glow_amount * influence;
            t_halation += m.halation_amount * influence;
            t_flare += m.flare_amount * influence;
            t_hue += m.hue * influence;
            t_sharpness += m.sharpness * influence;

            h0_h += m.hsl[0].hue * influence; h0_s += m.hsl[0].saturation * influence; h0_l += m.hsl[0].luminance * influence;
            h1_h += m.hsl[1].hue * influence; h1_s += m.hsl[1].saturation * influence; h1_l += m.hsl[1].luminance * influence;
            h2_h += m.hsl[2].hue * influence; h2_s += m.hsl[2].saturation * influence; h2_l += m.hsl[2].luminance * influence;
            h3_h += m.hsl[3].hue * influence; h3_s += m.hsl[3].saturation * influence; h3_l += m.hsl[3].luminance * influence;
            h4_h += m.hsl[4].hue * influence; h4_s += m.hsl[4].saturation * influence; h4_l += m.hsl[4].luminance * influence;
            h5_h += m.hsl[5].hue * influence; h5_s += m.hsl[5].saturation * influence; h5_l += m.hsl[5].luminance * influence;
            h6_h += m.hsl[6].hue * influence; h6_s += m.hsl[6].saturation * influence; h6_l += m.hsl[6].luminance * influence;
            h7_h += m.hsl[7].hue * influence; h7_s += m.hsl[7].saturation * influence; h7_l += m.hsl[7].luminance * influence;
        }
    }

    let final_hsl = array<HslColor, 8>(
        HslColor(h0_h, h0_s, h0_l, 0.0), HslColor(h1_h, h1_s, h1_l, 0.0),
        HslColor(h2_h, h2_s, h2_l, 0.0), HslColor(h3_h, h3_s, h3_l, 0.0),
        HslColor(h4_h, h4_s, h4_l, 0.0), HslColor(h5_h, h5_s, h5_l, 0.0),
        HslColor(h6_h, h6_s, h6_l, 0.0), HslColor(h7_h, h7_s, h7_l, 0.0)
    );

    initial_linear_rgb = apply_noise_reduction(
        initial_linear_rgb, absolute_coord_i,
        t_luma_nr, t_color_nr, scale, is_raw
    );

    let sharpness_blurred = textureLoad(sharpness_blur_texture, id.xy, 0).rgb;
    let tonal_blurred = textureLoad(tonal_blur_texture, id.xy, 0).rgb;
    let clarity_blurred = textureLoad(clarity_blur_texture, id.xy, 0).rgb;
    let structure_blurred = textureLoad(structure_blur_texture, id.xy, 0).rgb;

    var locally_contrasted_rgb = initial_linear_rgb;

    locally_contrasted_rgb = apply_sharpen(
        locally_contrasted_rgb,
        sharpness_blurred, tonal_blurred,
        absolute_coord_i, t_sharpness, t_sharp_thresh, is_raw
    );

    locally_contrasted_rgb = apply_local_contrast(locally_contrasted_rgb, clarity_blurred, t_clarity, is_raw, 1u, 0.0);
    locally_contrasted_rgb = apply_local_contrast(locally_contrasted_rgb, structure_blurred, t_structure, is_raw, 1u, 0.0);
    locally_contrasted_rgb = apply_centre_local_contrast(locally_contrasted_rgb, adjustments.global.centre, absolute_coord_i, clarity_blurred, is_raw);

    var processed_rgb = apply_linear_exposure(locally_contrasted_rgb, t_exposure);

    if (t_glow > 0.0) {
        processed_rgb = apply_glow_bloom(
            processed_rgb, structure_blurred, t_glow, is_raw,
            t_exposure, t_brightness, t_contrast, t_whites
        );
    }
    if (t_halation > 0.0) {
        processed_rgb = apply_halation(
            processed_rgb, clarity_blurred, t_halation, is_raw,
            t_exposure, t_brightness, t_contrast, t_whites
        );
    }
    if (t_flare > 0.0) {
        let uv = vec2<f32>(absolute_coord) / full_dims;
        var flare_color = textureSampleLevel(flare_texture, flare_sampler, uv, 0.0).rgb;
        flare_color *= 1.4;
        flare_color = flare_color * flare_color;
        let linear_luma = get_luma(max(processed_rgb, vec3<f32>(0.0)));
        var perceptual_luma: f32;
        if (linear_luma <= 1.0) {
            perceptual_luma = pow(max(linear_luma, 0.0), 1.0 / 2.2);
        } else {
            perceptual_luma = 1.0 + pow(linear_luma - 1.0, 1.0 / 2.2);
        }
        let protection = 1.0 - smoothstep(0.7, 1.8, perceptual_luma);
        processed_rgb += flare_color * t_flare * protection;
    }

    var composite_rgb_linear = apply_dehaze(processed_rgb, structure_blurred, is_raw, t_dehaze);
    composite_rgb_linear = apply_white_balance(composite_rgb_linear, t_temperature, t_tint);
    composite_rgb_linear = apply_centre_tonal_and_color(composite_rgb_linear, adjustments.global.centre, absolute_coord_i);
    composite_rgb_linear = apply_filmic_exposure(composite_rgb_linear, t_brightness);
    composite_rgb_linear = apply_tonal_adjustments(composite_rgb_linear, tonal_blurred, is_raw, t_contrast, t_shadows, t_whites, t_blacks);
    composite_rgb_linear = apply_highlights_adjustment(composite_rgb_linear, tonal_blurred, is_raw, t_highlights);
    composite_rgb_linear = apply_color_calibration(composite_rgb_linear, adjustments.global.color_calibration);
    // Band identity is read from a neighbourhood rather than a lone pixel. The
    // blur is of the unadjusted image, so it is white balanced to match; the
    // tonal work between them barely moves hue.
    var band_reference = clarity_blurred;
    if (is_raw == 0u) {
        band_reference = input_to_working(band_reference);
    }
    band_reference = apply_white_balance(band_reference, t_temperature, t_tint);

    composite_rgb_linear = apply_oklch_color(
        composite_rgb_linear,
        band_reference,
        t_hue,
        t_saturation,
        t_vibrance,
        final_hsl
    );

    composite_rgb_linear = apply_color_grading(
        composite_rgb_linear,
        adjustments.global.color_grading_shadows,
        adjustments.global.color_grading_midtones,
        adjustments.global.color_grading_highlights,
        adjustments.global.color_grading_global,
        adjustments.global.color_grading_blending,
        adjustments.global.color_grading_balance
    );

    for (var i = 0u; i < adjustments.mask_count; i = i + 1u) {
        let influence = get_mask_influence(i, absolute_coord);
        if (influence > 0.001) {
            let m = adjustments.mask_adjustments[i];
            let mask_graded = apply_color_grading(
                composite_rgb_linear,
                m.color_grading_shadows, m.color_grading_midtones, m.color_grading_highlights, m.color_grading_global, m.color_grading_blending, m.color_grading_balance
            );
            composite_rgb_linear = mix(composite_rgb_linear, mask_graded, influence);
        }
    }

    if (adjustments.global.vignette_amount != 0.0) {
        let full_dims_f = vec2<f32>(textureDimensions(input_texture));
        let coord_f = vec2<f32>(absolute_coord);
        let v_amount = adjustments.global.vignette_amount;
        let v_mid = adjustments.global.vignette_midpoint;
        let v_round = 1.0 - adjustments.global.vignette_roundness;
        let v_feather = adjustments.global.vignette_feather * 0.5;
        let aspect = full_dims_f.y / full_dims_f.x;
        let uv_centered = (coord_f / full_dims_f - 0.5) * 2.0;
        let uv_round = sign(uv_centered) * pow(abs(uv_centered), vec2<f32>(v_round, v_round));
        let d = length(uv_round * vec2<f32>(1.0, aspect)) * 0.5;
        let vignette_mask = smoothstep(v_mid - v_feather, v_mid + v_feather, d);
        if (v_amount < 0.0) {
            composite_rgb_linear *= (1.0 + v_amount * vignette_mask);
        } else {
            composite_rgb_linear = mix(composite_rgb_linear, vec3<f32>(1.0), v_amount * vignette_mask);
        }
    }

    // AgX and the sRGB transfer functions below are all defined against sRGB
    // primaries, so the working space resolves once here rather than inside each
    // of them. Phase 3 moves AgX into the working space and replaces this clip
    // with real gamut compression.
    let display_linear = gamut_clip_srgb(prophoto_to_srgb(composite_rgb_linear));

    var default_tonemapped: vec3<f32>;
    if (adjustments.global.tonemapper_mode == 1u) {
        default_tonemapped = agx_full_transform(display_linear);
    } else if (is_raw == 1u) {
        var srgb_emulated = linear_to_srgb(display_linear);
        const BRIGHTNESS_GAMMA: f32 = 1.1;
        srgb_emulated = pow(srgb_emulated, vec3<f32>(1.0 / BRIGHTNESS_GAMMA));
        const CONTRAST_MIX: f32 = 0.75;
        let contrast_curve = srgb_emulated * srgb_emulated * (3.0 - 2.0 * srgb_emulated);
        default_tonemapped = mix(srgb_emulated, contrast_curve, CONTRAST_MIX);
    } else {
        default_tonemapped = linear_to_srgb(display_linear);
    }
    var base_srgb: vec3<f32>;
    let is_scene_lut = (adjustments.global.has_lut == 1u && adjustments.global.lut_is_scene_referred == 1u);

    if (is_scene_lut) {
        let vlog_encoded = linear_to_vlog(display_linear);
        let lut_color = sample_lut_tetrahedral(vlog_encoded);
        base_srgb = mix(default_tonemapped, lut_color, adjustments.global.lut_intensity);
    } else {
        base_srgb = default_tonemapped;
    }

    var final_rgb = apply_all_curves(base_srgb,
        adjustments.global.luma_curve, adjustments.global.luma_curve_count,
        adjustments.global.red_curve, adjustments.global.red_curve_count,
        adjustments.global.green_curve, adjustments.global.green_curve_count,
        adjustments.global.blue_curve, adjustments.global.blue_curve_count
    );

    for (var i = 0u; i < adjustments.mask_count; i = i + 1u) {
        let influence = get_mask_influence(i, absolute_coord);
        if (influence > 0.001) {
            let m = adjustments.mask_adjustments[i];
            let mask_curved_srgb = apply_all_curves(final_rgb,
                m.luma_curve, m.luma_curve_count,
                m.red_curve, m.red_curve_count,
                m.green_curve, m.green_curve_count,
                m.blue_curve, m.blue_curve_count
            );
            final_rgb = mix(final_rgb, mask_curved_srgb, influence);
        }
    }

    if (adjustments.global.has_lut == 1u && adjustments.global.lut_is_scene_referred == 0u) {
        let lut_color = sample_lut_tetrahedral(final_rgb);
        final_rgb = mix(final_rgb, lut_color, adjustments.global.lut_intensity);
    }

    if (adjustments.global.grain_amount > 0.0) {
        let coord = vec2<f32>(absolute_coord_i);
        let amount = adjustments.global.grain_amount * 0.5;
        let grain_frequency = (1.0 / max(adjustments.global.grain_size, 0.1)) / scale;
        let roughness = adjustments.global.grain_roughness;
        let luma = max(0.0, get_display_luma(final_rgb));
        let luma_mask = smoothstep(0.0, 0.15, luma) * (1.0 - smoothstep(0.6, 1.0, luma));
        let base_coord = coord * grain_frequency;
        let rough_coord = coord * grain_frequency * 0.6;
        let noise_base = gradient_noise(base_coord);
        let noise_rough = gradient_noise(rough_coord + vec2<f32>(5.2, 1.3));
        let noise_val = mix(noise_base, noise_rough, roughness);
        final_rgb += vec3<f32>(noise_val) * amount * luma_mask;
    }

    if (adjustments.global.show_clipping == 1u) {
        let HIGHLIGHT_WARNING_COLOR = vec3<f32>(1.0, 0.0, 0.0);
        let SHADOW_WARNING_COLOR = vec3<f32>(0.0, 0.0, 1.0);
        let HIGHLIGHT_CLIP_THRESHOLD = 0.998;
        let SHADOW_CLIP_THRESHOLD = 0.002;
        if (any(final_rgb > vec3<f32>(HIGHLIGHT_CLIP_THRESHOLD))) {
            final_rgb = HIGHLIGHT_WARNING_COLOR;
        } else if (any(final_rgb < vec3<f32>(SHADOW_CLIP_THRESHOLD))) {
            final_rgb = SHADOW_WARNING_COLOR;
        }
    }

    let dither_amount = 1.0 / 255.0;
    final_rgb += dither(id.xy) * dither_amount;

    textureStore(output_texture, id.xy, vec4<f32>(clamp(final_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), original_alpha));
}
