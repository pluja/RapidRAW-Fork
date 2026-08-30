//! Reads constants and function bodies out of shaders/shader.wgsl.
//!
//! Several modules here exist only to make the shader's maths testable, and
//! carry a Rust copy of what the shader runs. A copy asserted against another
//! Rust literal proves nothing: the shader is the live path, and a mirror that
//! has drifted from it passes every such test. Everything in this module reads
//! the shader source, so drift fails.
//!
//! The `_in` variants take the source to read, so this module's own tests can
//! exercise the parsers against a fixture rather than against live constants,
//! which would make an ordinary shader edit fail here as well as where it means
//! something.

use crate::color_space::Mat3;

pub fn source() -> &'static str {
    include_str!("shaders/shader.wgsl")
}

/// Reads a `const NAME: f32 = VALUE;`.
pub fn f32_const(name: &str) -> f32 {
    f32_const_in(source(), name)
}

/// Reads a `const NAME = vec3<f32>(x, y, z);`.
pub fn vec3_const(name: &str) -> [f32; 3] {
    vec3_const_in(source(), name)
}

/// Reads a `const NAME = mat3x3<f32>(...)` into row-major order.
pub fn mat3_const(name: &str) -> Mat3 {
    mat3_const_in(source(), name)
}

/// Reads a `const NAME = array<f32, N>(...);`, which may span several lines.
pub fn f32_array_const<const N: usize>(name: &str) -> [f32; N] {
    f32_array_const_in(source(), name)
}

/// The body of `fn NAME`, for asserting that the shader applies a constant
/// rather than merely defining one that matches.
pub fn fn_body(name: &str) -> &'static str {
    fn_body_in(source(), name)
}

fn f32_const_in(src: &str, name: &str) -> f32 {
    let decl = declaration(src, name);
    let values = floats(decl);
    match values[..] {
        [only] => only,
        _ => panic!(
            "`const {name}` holds {} values, not one: {decl:?}",
            values.len()
        ),
    }
}

fn vec3_const_in(src: &str, name: &str) -> [f32; 3] {
    let values = floats(constructor_payload(name, declaration(src, name)));
    values
        .as_slice()
        .try_into()
        .unwrap_or_else(|_| panic!("`const {name}` holds {} values, not three", values.len()))
}

/// WGSL's mat3x3 constructor takes its arguments as columns, so the three vec3s
/// in the source are the matrix's columns. Mat3 here is row-major, which is why
/// this transposes rather than copying straight through.
fn mat3_const_in(src: &str, name: &str) -> Mat3 {
    let columns = floats(constructor_payload(name, declaration(src, name)));
    assert_eq!(
        columns.len(),
        9,
        "`const {name}` holds {} values, not nine",
        columns.len()
    );
    std::array::from_fn(|row| std::array::from_fn(|column| columns[column * 3 + row]))
}

fn f32_array_const_in<const N: usize>(src: &str, name: &str) -> [f32; N] {
    let values = floats(constructor_payload(name, declaration(src, name)));
    values
        .as_slice()
        .try_into()
        .unwrap_or_else(|_| panic!("`const {name}` holds {} values, not {N}", values.len()))
}

fn fn_body_in<'a>(src: &'a str, name: &str) -> &'a str {
    let needle = format!("fn {name}");
    let after = src
        .split(&needle)
        .nth(1)
        .unwrap_or_else(|| panic!("shaders/shader.wgsl has no `fn {name}`"));
    &after[..after.find("\n}").unwrap_or(after.len())]
}

/// Everything between `const NAME` and the `;` that ends the declaration.
fn declaration<'a>(src: &'a str, name: &str) -> &'a str {
    const KEYWORD: &str = "const ";
    let mut searched = 0;

    while let Some(found) = src[searched..].find(KEYWORD) {
        let start = searched + found;
        searched = start + KEYWORD.len();

        // A `const` inside a comment or mid-line is not a declaration.
        let line_is_blank_before = src[..start]
            .chars()
            .rev()
            .take_while(|c| *c != '\n')
            .all(char::is_whitespace);
        if !line_is_blank_before {
            continue;
        }

        let rest = &src[searched..];
        let ident = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if &rest[..ident] != name {
            continue;
        }

        let end = rest
            .find(';')
            .unwrap_or_else(|| panic!("`const {name}` never ends in a semicolon"));
        return &rest[..end];
    }

    panic!("shaders/shader.wgsl has no `const {name}`")
}

/// The arguments of the constructor a declaration is built from, so that the
/// element count in `array<f32, 8>` is not mistaken for a value.
fn constructor_payload<'a>(name: &str, decl: &'a str) -> &'a str {
    let open = decl
        .find('(')
        .unwrap_or_else(|| panic!("`const {name}` is not built from a constructor: {decl:?}"));
    let close = decl[open..]
        .rfind(')')
        .unwrap_or_else(|| panic!("`const {name}` has no closing parenthesis: {decl:?}"));
    &decl[open + 1..open + close]
}

/// Every float literal in a fragment of WGSL, in source order.
///
/// A digit that continues an identifier is part of a type rather than a value,
/// which is what keeps the 3 and the 32 of `vec3<f32>` out of the results.
fn floats(text: &str) -> Vec<f32> {
    let bytes = text.as_bytes();
    let mut values = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let follows_identifier = i > 0 && {
            let previous = bytes[i - 1] as char;
            previous.is_alphanumeric() || previous == '_'
        };
        let digit_here = (bytes[i] as char).is_ascii_digit();
        let signed_digit = matches!(bytes[i], b'-' | b'+' | b'.')
            && bytes
                .get(i + 1)
                .is_some_and(|next| (*next as char).is_ascii_digit());

        if follows_identifier || !(digit_here || signed_digit) {
            i += 1;
            continue;
        }

        let start = i;
        i += 1;
        while i < bytes.len() {
            let c = bytes[i] as char;
            let exponent_sign = matches!(c, '-' | '+') && matches!(bytes[i - 1] as char, 'e' | 'E');
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || exponent_sign {
                i += 1;
            } else {
                break;
            }
        }

        let token = &text[start..i];
        values.push(
            token
                .parse::<f32>()
                .unwrap_or_else(|_| panic!("could not read a number from {token:?}")),
        );
    }

    values
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
// const COMMENTED_OUT: f32 = 9.0;
const SCALAR: f32 = 0.18;
const EXPONENT: f32 = 1.5e-3;
const WEIGHTS = vec3<f32>(0.2126, 0.7152, 0.0722);
const WEIGHTS_B = vec3<f32>(1.0, 2.0, 3.0);
const COLUMNS = mat3x3<f32>(
    vec3<f32>(1.0, 2.0, 3.0),
    vec3<f32>(4.0, 5.0, 6.0),
    vec3<f32>(7.0, 8.0, 9.0),
);
const INLINE_LIST = array<f32, 4>(10.0, 20.0, 30.0, 40.0);
const WRAPPED_LIST = array<f32, 4>(
    -1.0, 2.5, -3.25, 4.0
);

fn shaped(x: f32) -> f32 {
    return x * SCALAR;
}
";

    #[test]
    fn reads_a_scalar() {
        assert_eq!(f32_const_in(FIXTURE, "SCALAR"), 0.18);
        assert_eq!(f32_const_in(FIXTURE, "EXPONENT"), 1.5e-3);
    }

    #[test]
    fn reads_a_vector_without_its_type() {
        assert_eq!(vec3_const_in(FIXTURE, "WEIGHTS"), [0.2126, 0.7152, 0.0722]);
    }

    /// WGSL builds a matrix from columns and Mat3 stores rows, so a reader that
    /// copies straight through transposes every matrix in the pipeline.
    #[test]
    fn reads_a_matrix_by_rows() {
        assert_eq!(
            mat3_const_in(FIXTURE, "COLUMNS"),
            [[1.0, 4.0, 7.0], [2.0, 5.0, 8.0], [3.0, 6.0, 9.0]]
        );
    }

    #[test]
    fn reads_an_array_without_its_length() {
        assert_eq!(
            f32_array_const_in::<4>(FIXTURE, "INLINE_LIST"),
            [10.0, 20.0, 30.0, 40.0]
        );
        assert_eq!(
            f32_array_const_in::<4>(FIXTURE, "WRAPPED_LIST"),
            [-1.0, 2.5, -3.25, 4.0]
        );
    }

    #[test]
    fn reads_a_function_body() {
        assert!(fn_body_in(FIXTURE, "shaped").contains("x * SCALAR"));
    }

    /// A prefix must not satisfy a longer name, or PROPHOTO_TO_SRGB_R0 would
    /// answer for R1 and R2 as well.
    #[test]
    fn matches_a_whole_name() {
        assert_eq!(vec3_const_in(FIXTURE, "WEIGHTS_B"), [1.0, 2.0, 3.0]);
    }

    #[test]
    #[should_panic(expected = "no `const COMMENTED_OUT`")]
    fn ignores_a_commented_declaration() {
        f32_const_in(FIXTURE, "COMMENTED_OUT");
    }

    #[test]
    #[should_panic(expected = "no `const NOT_A_REAL_CONSTANT`")]
    fn a_missing_constant_is_loud() {
        f32_const("NOT_A_REAL_CONSTANT");
    }

    /// The shader is otherwise only compiled when a pipeline is built, which
    /// needs a GPU, so a syntax or type error reaches a user's machine rather
    /// than a test run.
    #[test]
    fn every_shader_compiles() {
        for (name, src) in [
            ("shader.wgsl", source()),
            ("blur.wgsl", include_str!("shaders/blur.wgsl")),
            ("display.wgsl", include_str!("shaders/display.wgsl")),
            ("flare.wgsl", include_str!("shaders/flare.wgsl")),
        ] {
            let module = wgpu::naga::front::wgsl::parse_str(src)
                .unwrap_or_else(|e| panic!("{name} does not parse:\n{}", e.emit_to_string(src)));

            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name} does not validate: {e:?}"));
        }
    }
}
