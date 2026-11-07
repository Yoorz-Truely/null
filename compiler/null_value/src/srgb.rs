//! Null's sRGB rule (spec, section 6; Sau's decision S62): `#rrggbb` is the IEC 61966-2-1 sRGB
//! curve, exactly rounded once into float32, by a fixed table of the 256 channel values, so it is
//! the same on every platform. No floating-point `pow` is involved; the test proves every entry
//! with exact integer arithmetic (`tests::every_entry_is_the_exactly_rounded_curve`).

/// `TABLE[i]` is the float32, as bits, nearest to the linear value of the sRGB channel `i / 255`:
/// `c / 12.92` for `c <= 0.04045`, else `((c + 0.055) / 1.055)^2.4`.
pub(crate) const TABLE: [u32; 256] = [
    0x00000000, 0x399f22b4, 0x3a1f22b4, 0x3a6eb40e, 0x3a9f22b4, 0x3ac6eb61, 0x3aeeb40e, 0x3b0b3e5d,
    0x3b1f22b4, 0x3b33070a, 0x3b46eb61, 0x3b5b518e, 0x3b70f18f, 0x3b83e1c6, 0x3b8fe616, 0x3b9c87fd,
    0x3ba9c9b6, 0x3bb7ad6f, 0x3bc6354a, 0x3bd56360, 0x3be539c1, 0x3bf5ba71, 0x3c0373b6, 0x3c0c6153,
    0x3c15a705, 0x3c1f45be, 0x3c293e6b, 0x3c3391f7, 0x3c3e4149, 0x3c494d44, 0x3c54b6c9, 0x3c607eb4,
    0x3c6ca5df, 0x3c792d22, 0x3c830aa9, 0x3c89af9f, 0x3c9085dc, 0x3c978dc6, 0x3c9ec7c2, 0x3ca63433,
    0x3cadd37d, 0x3cb5a602, 0x3cbdac21, 0x3cc5e63a, 0x3cce54ac, 0x3cd6f7d5, 0x3cdfd010, 0x3ce8ddba,
    0x3cf2212d, 0x3cfb9ac3, 0x3d02a56a, 0x3d0798dd, 0x3d0ca7e6, 0x3d11d2af, 0x3d171964, 0x3d1c7c30,
    0x3d21fb3c, 0x3d2796b2, 0x3d2d4ebb, 0x3d332381, 0x3d39152b, 0x3d3f23e4, 0x3d454fd2, 0x3d4b991d,
    0x3d51ffec, 0x3d588468, 0x3d5f26b6, 0x3d65e6fd, 0x3d6cc563, 0x3d73c20e, 0x3d7add24, 0x3d810b65,
    0x3d84b793, 0x3d88732e, 0x3d8c3e48, 0x3d9018f4, 0x3d940344, 0x3d97fd49, 0x3d9c0715, 0x3da020ba,
    0x3da44a4a, 0x3da883d6, 0x3daccd6f, 0x3db12727, 0x3db5910f, 0x3dba0b38, 0x3dbe95b3, 0x3dc33090,
    0x3dc7dbe0, 0x3dcc97b4, 0x3dd1641d, 0x3dd6412b, 0x3ddb2eee, 0x3de02d76, 0x3de53cd4, 0x3dea5d18,
    0x3def8e51, 0x3df4d090, 0x3dfa23e5, 0x3dff885e, 0x3e027f06, 0x3e05427f, 0x3e080ea2, 0x3e0ae377,
    0x3e0dc104, 0x3e10a753, 0x3e13966a, 0x3e168e51, 0x3e198f0f, 0x3e1c98ac, 0x3e1fab30, 0x3e22c6a1,
    0x3e25eb07, 0x3e29186a, 0x3e2c4ed0, 0x3e2f8e42, 0x3e32d6c5, 0x3e362862, 0x3e39831f, 0x3e3ce703,
    0x3e405417, 0x3e43ca60, 0x3e4749e6, 0x3e4ad2af, 0x3e4e64c3, 0x3e520029, 0x3e55a4e7, 0x3e595305,
    0x3e5d0a89, 0x3e60cb7a, 0x3e6495df, 0x3e6869be, 0x3e6c471f, 0x3e702e07, 0x3e741e7e, 0x3e78188b,
    0x3e7c1c33, 0x3e8014bf, 0x3e822039, 0x3e84308b, 0x3e8645b8, 0x3e885fc3, 0x3e8a7eb0, 0x3e8ca281,
    0x3e8ecb3b, 0x3e90f8df, 0x3e932b72, 0x3e9562f6, 0x3e979f6f, 0x3e99e0e0, 0x3e9c274c, 0x3e9e72b6,
    0x3ea0c321, 0x3ea31890, 0x3ea57307, 0x3ea7d288, 0x3eaa3716, 0x3eaca0b6, 0x3eaf0f68, 0x3eb18332,
    0x3eb3fc15, 0x3eb67a14, 0x3eb8fd34, 0x3ebb8576, 0x3ebe12de, 0x3ec0a56e, 0x3ec33d2a, 0x3ec5da14,
    0x3ec87c30, 0x3ecb2380, 0x3ecdd008, 0x3ed081ca, 0x3ed338c9, 0x3ed5f508, 0x3ed8b68a, 0x3edb7d52,
    0x3ede4963, 0x3ee11abf, 0x3ee3f169, 0x3ee6cd65, 0x3ee9aeb5, 0x3eec955b, 0x3eef815c, 0x3ef272b8,
    0x3ef56974, 0x3ef86593, 0x3efb6716, 0x3efe6e00, 0x3f00bd2b, 0x3f02460c, 0x3f03d1a5, 0x3f055ff7,
    0x3f06f104, 0x3f0884cd, 0x3f0a1b54, 0x3f0bb499, 0x3f0d509f, 0x3f0eef65, 0x3f1090ef, 0x3f12353d,
    0x3f13dc50, 0x3f15862a, 0x3f1732cc, 0x3f18e237, 0x3f1a946e, 0x3f1c4970, 0x3f1e0140, 0x3f1fbbde,
    0x3f21794d, 0x3f23398c, 0x3f24fc9f, 0x3f26c285, 0x3f288b41, 0x3f2a56d2, 0x3f2c253c, 0x3f2df67f,
    0x3f2fca9c, 0x3f31a194, 0x3f337b6a, 0x3f35581d, 0x3f3737b0, 0x3f391a24, 0x3f3aff7a, 0x3f3ce7b2,
    0x3f3ed2cf, 0x3f40c0d2, 0x3f42b1bc, 0x3f44a58e, 0x3f469c49, 0x3f4895ef, 0x3f4a9280, 0x3f4c91ff,
    0x3f4e946c, 0x3f5099c9, 0x3f52a216, 0x3f54ad56, 0x3f56bb88, 0x3f58ccaf, 0x3f5ae0cc, 0x3f5cf7df,
    0x3f5f11ea, 0x3f612eef, 0x3f634eee, 0x3f6571e9, 0x3f6797e0, 0x3f69c0d5, 0x3f6becca, 0x3f6e1bbf,
    0x3f704db5, 0x3f7282ae, 0x3f74baab, 0x3f76f5ae, 0x3f7933b6, 0x3f7b74c6, 0x3f7db8de, 0x3f800000,
];

/// The scene-linear value of the sRGB channel `byte` (0 to 255).
pub fn srgb_to_linear(byte: u8) -> f32 {
    f32::from_bits(TABLE[usize::from(byte)])
}

/// `#rrggbb` as three scene-linear channels, or `None` if `word` is not six hexadecimal digits
/// after `#`.
pub fn read_srgb(word: &str) -> Option<[f32; 3]> {
    let hex = word.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16)
            .ok()
            .map(srgb_to_linear)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// The `#rrggbb` that reads back to exactly these channels (a fourth, alpha, must be exactly 1),
/// or `None`: a colour is printed as `#rrggbb` only if it converts to the same bits (spec,
/// section 6), otherwise as its list of floats.
pub fn print_srgb(channels: &[f32]) -> Option<String> {
    let (rgb, alpha) = match channels {
        [r, g, b] => ([*r, *g, *b], None),
        [r, g, b, a] => ([*r, *g, *b], Some(*a)),
        _ => return None,
    };
    if alpha.is_some_and(|a| a.to_bits() != 1.0f32.to_bits()) {
        return None;
    }
    let mut out = String::from("#");
    for c in rgb {
        let byte = TABLE.iter().position(|&t| t == c.to_bits())?;
        out.push_str(&format!("{byte:02x}"));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    //! The table proven entry by entry, with integers only.
    use num_bigint::BigUint;

    use super::*;

    /// A non-negative finite float32 as `n / 2^150`, exactly (every float32 is a whole number of
    /// units of 2^-149; one more power of two leaves room for a midpoint).
    fn scaled(bits: u32) -> BigUint {
        let x = f32::from_bits(bits);
        assert!(x >= 0.0 && x.is_finite());
        let exponent = (bits >> 23) & 0xff;
        let mantissa = bits & 0x7f_ffff;
        let (m, e) = if exponent == 0 {
            (mantissa, -149i32)
        } else {
            (mantissa | 0x80_0000, exponent as i32 - 150)
        };
        BigUint::from(m) << usize::try_from(e + 150).unwrap()
    }

    /// The midpoint between float32s `a` and `b` as `n / 2^151`.
    fn midpoint(a: u32, b: u32) -> BigUint {
        scaled(a) + scaled(b)
    }

    /// Whether the value `m / 2^151` is below the curve at channel `i`, compared exactly:
    /// below `i / (255 * 12.92)` on the linear part, and, on the power part, `m^5` below `r^12`
    /// with `r = (i/255 + 0.055) / 1.055 = (40 i + 561) / 10761`.
    fn below(m: &BigUint, i: u32) -> bool {
        let two151 = BigUint::from(1u8) << 151usize;
        if u64::from(i) * 100_000 <= 4045 * 255 {
            // m / 2^151 < i / (255 * 12.92) = 100 i / 329460.
            m * BigUint::from(329_460u32) < BigUint::from(100 * i) * &two151
        } else {
            let num = BigUint::from(40 * i + 561);
            let den = BigUint::from(10_761u32);
            m.pow(5) * den.pow(12) < num.pow(12) * two151.pow(5)
        }
    }

    #[test]
    fn every_entry_is_the_exactly_rounded_curve() {
        assert_eq!(TABLE[0], 0, "black is 0");
        assert_eq!(TABLE[255], 1.0f32.to_bits(), "white is 1");
        for i in 1..255u32 {
            let t = TABLE[i as usize];
            // The curve lies strictly between the midpoints below and above the entry, so the
            // entry is its nearest float32 (no ties can occur: the curve is irrational there,
            // and on the linear part checked with strict comparisons both ways).
            let lo = midpoint(t - 1, t);
            let hi = midpoint(t, t + 1);
            assert!(
                below(&lo, i),
                "channel {i}: the curve is below the entry's lower midpoint"
            );
            assert!(
                !below(&hi, i) && hi != lo,
                "channel {i}: the curve is above the upper midpoint"
            );
            // And the table rises.
            assert!(TABLE[i as usize - 1] < t);
        }
    }

    #[test]
    fn hex_reads_through_the_table_and_prints_back() {
        assert_eq!(read_srgb("#000000"), Some([0.0; 3]));
        assert_eq!(read_srgb("#ffffff"), Some([1.0; 3]));
        let grey = read_srgb("#808080").unwrap();
        assert_eq!(grey[0].to_bits(), 0x3e5d_0a89);
        assert_eq!(print_srgb(&grey).as_deref(), Some("#808080"));
        assert_eq!(
            print_srgb(&[grey[0], grey[1], grey[2], 1.0]).as_deref(),
            Some("#808080")
        );
        assert_eq!(print_srgb(&[grey[0], grey[1], grey[2], 0.5]), None);
        assert_eq!(read_srgb("#80808"), None);
        assert_eq!(read_srgb("808080"), None);
        assert_eq!(read_srgb("#80808g"), None);
    }

    /// Blender's colour picker gives `#808080` as 0.21586056 (its OCIO conversion, measured in
    /// Sau, task 1c-01f), not the curve's exactly rounded 0.21586050. Such a value does not read
    /// back from any `#rrggbb`, so it prints as its exact list of floats (S62).
    #[test]
    fn a_picker_value_is_not_hex() {
        let picker = 0.215_860_56_f32;
        assert_ne!(picker.to_bits(), TABLE[128]);
        assert_eq!(print_srgb(&[picker; 3]), None);
        assert_eq!(crate::print(picker, None), "0.21586056");
    }
}
