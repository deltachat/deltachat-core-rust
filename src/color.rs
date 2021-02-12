//! Implementation of Consistent Color Generation
//!
//! Consistent Color Generation is defined in XEP-0392.
//!
//! Color Vision Deficiency correction is not implemented as Delta Chat does not offer
//! corresponding settings.
use hsluv::hsluv_to_rgb;
use sha1::{Digest, Sha1};

/// Converts an identifier to Hue angle.
fn str_to_angle(s: impl AsRef<str>) -> f64 {
    let bytes = s.as_ref().as_bytes();
    let result = Sha1::digest(bytes);
    let checksum: u16 = result.get(0).map_or(0, |&x| u16::from(x))
        + 256 * result.get(1).map_or(0, |&x| u16::from(x));
    f64::from(checksum) / 65536.0 * 360.0
}

/// Converts an identifier to RGB color.
///
/// Returns a 24-bit number with 8 least significant bits corresponding to the blue color and 8
/// most significant bits corresponding to the red color.
///
/// Saturation is set to maximum (100.0) to make colors distinguishable, and lightness is set to
/// half (50.0) to make colors suitable both for light and dark theme.
pub(crate) fn str_to_color(s: impl AsRef<str>) -> u32 {
    let (r, g, b) = hsluv_to_rgb((str_to_angle(s), 100.0, 50.0));
    65536 * (r * 256.0) as u32 + 256 * (g * 256.0) as u32 + (b * 256.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::float_cmp)]
    #[test]
    fn test_str_to_angle() {
        // Test against test vectors from
        // https://xmpp.org/extensions/xep-0392.html#testvectors-fullrange-no-cvd
        assert!((str_to_angle("Romeo") - 327.255249).abs() < 1e-6);
        assert!((str_to_angle("juliet@capulet.lit") - 209.410400).abs() < 1e-6);
        assert!((str_to_angle("😺") - 331.199341).abs() < 1e-6);
        assert!((str_to_angle("council") - 359.994507).abs() < 1e-6);
        assert!((str_to_angle("Board") - 171.430664).abs() < 1e-6);
    }
}
