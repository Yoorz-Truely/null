//! A datablock's display name as Blender holds it (Sau's decision S72, task 1c-02i): Blender
//! strips every byte its UTF-8 check refuses from a name (`BKE_id_new_name_validate` calls
//! `BLI_str_utf8_invalid_strip`), so a name is UTF-8, and the name a byte list would become is
//! what that strip leaves.
//!
//! Blender's check (`BLI_str_utf8_invalid_byte`, from libswish3 after PCRE) is not Rust's. It
//! takes 5- and 6-byte sequences and code points past U+10FFFF, takes surrogates other than seven
//! listed sequences, and refuses U+FFFE and U+FFFF. Its strip drops one refused byte at a time
//! and checks again from there. [`blender_strip`] follows it exactly, with Blender's own test
//! table (`BLI_string_utf8_test.cc`) as the reference.

/// The bytes Blender keeps of `bytes` as a name: each byte its UTF-8 check refuses is dropped,
/// one at a time, as `BLI_str_utf8_invalid_strip` does.
pub fn blender_strip(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let mut from = 0;
    while let Some(bad) = invalid_byte(&out[from..]) {
        out.remove(from + bad);
        from += bad;
    }
    out
}

/// The index of the first byte Blender's UTF-8 check refuses, as `BLI_str_utf8_invalid_byte`
/// gives it: the first byte of the sequence that fails.
fn invalid_byte(s: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < s.len() {
        let start = i;
        let c = s[i];
        if matches!(c, 0x00 | 0xfe | 0xff) {
            return Some(start);
        }
        if c < 0x80 {
            i += 1;
            continue;
        }
        if c & 0xc0 != 0xc0 {
            return Some(start);
        }
        // The bytes after the first: 1 to 5.
        let after = match c {
            0xc0..=0xdf => 1,
            0xe0..=0xef => 2,
            0xf0..=0xf7 => 3,
            0xf8..=0xfb => 4,
            _ => 5,
        };
        if s.len() - start <= after {
            return Some(start);
        }
        let second = s[start + 1];
        if second & 0xc0 != 0x80 {
            return Some(start);
        }
        // Overlong forms, and the sequences Markus Kuhn's stress test (sections 5.1 to 5.3) lists.
        let third = s.get(start + 2).copied();
        let refused = match after {
            1 => c & 0x3e == 0,
            2 => {
                (c == 0xe0 && second & 0x20 == 0)
                    || (c == 0xed
                        && matches!(
                            (second, third),
                            (0xa0, Some(0x80))
                                | (0xad, Some(0xbf))
                                | (0xae, Some(0x80))
                                | (0xaf, Some(0xbf))
                                | (0xb0, Some(0x80))
                                | (0xbe, Some(0x80))
                                | (0xbf, Some(0xbf))
                        ))
                    || (c == 0xef && second == 0xbf && matches!(third, Some(0xbe | 0xbf)))
            }
            3 => c == 0xf0 && second & 0x30 == 0,
            4 => c == 0xf8 && second & 0x38 == 0,
            _ => c == 0xfc && second & 0x3c == 0,
        };
        if refused {
            return Some(start);
        }
        // Every byte after the second starts with the bits 10.
        if s[start + 2..start + 1 + after]
            .iter()
            .any(|b| b & 0xc0 != 0x80)
        {
            return Some(start);
        }
        i = start + 1 + after;
    }
    None
}

/// The name a byte list becomes in Blender, as Null can write it: [`blender_strip`], then any
/// sequence Blender keeps that is not Unicode (a surrogate, a 5- or 6-byte form, a code point past
/// U+10FFFF) dropped too, since a Null string is UTF-8.
pub fn utf8_reading(bytes: &[u8]) -> String {
    let kept = blender_strip(bytes);
    kept.utf8_chunks().map(|c| c.valid()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cases from Blender's `utf8_invalid_tests` (`BLI_string_utf8_test.cc`), without their
    /// padding: the bytes, and what the strip leaves.
    const BLENDERS: &[(&[u8], &[u8])] = &[
        (
            b"\xce\xba\xe1\xbd\xb9\xcf\x83\xce\xbc\xce\xb5",
            b"\xce\xba\xe1\xbd\xb9\xcf\x83\xce\xbc\xce\xb5",
        ),
        (b"\xc2\x80", b"\xc2\x80"),
        (b"\xe0\xa0\x80", b"\xe0\xa0\x80"),
        (b"\xf0\x90\x80\x80", b"\xf0\x90\x80\x80"),
        (b"\xf8\x88\x80\x80\x80", b"\xf8\x88\x80\x80\x80"),
        (b"\xfc\x84\x80\x80\x80\x80", b"\xfc\x84\x80\x80\x80\x80"),
        (b"\xef\xbf\xbf", b""),
        (b"\xf7\xbf\xbf\xbf", b"\xf7\xbf\xbf\xbf"),
        (b"\x80", b""),
        (b"\xc0 ", b" "),
        (b"\xe0\x80", b""),
        (b"\xc0\xaf", b""),
        (b"\xe0\x80\xaf", b""),
        (b"\xed\xa0\x80", b""),
        (b"\xed\xa1\x80", b"\xed\xa1\x80"),
        (b"\xef\xbf\xbe", b""),
        (b"\xfe", b""),
        (b"P\xfcr", b"Pr"),
    ];

    #[test]
    fn the_strip_is_blenders() {
        for (bytes, kept) in BLENDERS {
            assert_eq!(blender_strip(bytes), kept.to_vec(), "{bytes:x?}");
        }
    }

    #[test]
    fn valid_utf8_is_kept_whole() {
        for s in ["Pier", "Pür", "日本", "😀 ok", "a\u{d}b"] {
            assert_eq!(blender_strip(s.as_bytes()), s.as_bytes());
            assert_eq!(utf8_reading(s.as_bytes()), s);
        }
    }

    #[test]
    fn the_reading_drops_what_blender_drops_and_what_is_not_unicode() {
        assert_eq!(utf8_reading(b"P\xfcr"), "Pr");
        assert_eq!(utf8_reading(b"M\xfcller \xe9t\xe9"), "Mller t");
        // Blender keeps a lone surrogate; a Null string cannot hold one.
        assert_eq!(utf8_reading(b"a\xed\xa1\x80b"), "ab");
        assert_eq!(utf8_reading(b"\xfc\xfd"), "");
    }
}
