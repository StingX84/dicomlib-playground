//! GB18030, GBK and GB2312 forward/backward functions

mod index;
mod ranges;

use crate::tables::{BackwardResult, ForwardResult};
use index::GB18030_INDEX;
use ranges::GB18030_RANGES;

/// Unified chinese encoder/decoder mode selection
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Mode {
    /// 1, 2, 4 bytes mode with all the unicode
    Gb18030,
    /// 1, 2 bytes mode
    Gbk,
    /// Only 2-bytes G1 mode.
    Gb2312,
}

/// Translates 4-byte GBk/GB18030 code into Unicode
///
/// Modeled after: https://encoding.spec.whatwg.org/#index-gb18030-ranges-code-point
fn forward_whatwg_gb18030_ranges(pointer: u32) -> Option<u32> {
    // > 1. If pointer is greater than 39419 and less than 189000, or pointer is
    // >    greater than 1237575, return null.
    if (pointer > 39419 && pointer < 189000) || pointer > 1237575 {
        return None;
    }
    // > 2. If pointer is 7457, return code point U+E7C7.
    if pointer == 7457 {
        return Some(0xE7C7);
    }
    // > 3. Let offset be the last pointer in index gb18030 ranges that is less than
    // >    or equal to pointer and let code point offset be its corresponding code
    // >    point.
    GB18030_RANGES
        .iter()
        .rev()
        .find(|(p, _)| *p <= pointer)
        // > 4. Return a code point whose value is code point offset + pointer − offset.
        .map(|&(offset, code_offset)| code_offset + pointer - offset)
}

/// Translates Unicode to 4-byte GBK/GB18030
///
/// Modeled after: https://encoding.spec.whatwg.org/#index-gb18030-ranges-pointer
fn backward_whatwg_gb18030_ranges(code_point: u32) -> Option<u32> {
    // > 1. If code point is U+E7C7, return pointer 7457.
    if code_point == 0xE7C7 {
        return Some(7457);
    }
    // > 2. Let offset be the last code point in index gb18030 ranges that is less
    // >    than or equal to code point and let pointer offset be its corresponding
    // >    pointer.
    GB18030_RANGES
        .iter()
        .rev()
        .find(|(_, cp)| *cp <= code_point)
        // > 3. Return a pointer whose value is pointer offset + code point − offset.
        .map(|&(pointer_offset, offset)| pointer_offset + code_point - offset)
}

/// Translates 1, 2 or 4 bytes of input data to Unicode
///
/// Modeled after: https://encoding.spec.whatwg.org/#gb18030-decoder
fn forward_whatwg_gb18030(mode: Mode, c: &[u8]) -> ForwardResult {
    let mut first = 0x00u8;
    let mut second = 0x00u8;
    let mut third = 0x00u8;

    // Note: whatwg decoder described as "stream" version, so
    // we will "emulate" the stream of maximum 4 bytes.
    // All the comments starting with ">" are the quotes from
    // the whatwg site.
    for index in 0_usize..4 {
        // > 1. If byte is end-of-queue and gb18030 first, gb18030 second, and
        // >    gb18030 third are 0x00, return finished.
        // > 2. If byte is end-of-queue, and gb18030 first, gb18030 second, or
        // >    gb18030 third is not 0x00, set gb18030 first, gb18030 second,
        // >    and gb18030 third to 0x00, and return error.
        let Some(&byte) = c.get(index) else {
            // In our case, we may end up here ONLY if we've invoked on an empty
            // input or other code down below has not produced and output.
            return (index as u8, None);
        };

        // > 3. If gb18030 third is not 0x00, then:
        if third != 0 {
            // > 1. If byte is not in the range 0x30 to 0x39, inclusive, then:
            if !matches!(byte, 0x30..=0x39) {
                // > 1. Prepend gb18030 second, gb18030 third, and byte to ioQueue.
                // > 2. Set gb18030 first, gb18030 second, and gb18030 third to 0x00.
                // > 3. Return error.
                return (1, None);
            }

            // > 2. Let code point be the index gb18030 ranges code point for
            // >    ((gb18030 first − 0x81) × (10 × 126 × 10)) + ((gb18030
            // >    second − 0x30) × (10 × 126)) + ((gb18030 third − 0x81) × 10)
            // >    + byte − 0x30.

            let pointer = ((first - 0x81) as u32 * (10 * 126 * 10))
                + ((second - 0x30) as u32 * (10 * 126))
                + ((third - 0x81) as u32 * 10)
                + byte as u32
                - 0x30;
            let code_point = forward_whatwg_gb18030_ranges(pointer);
            // > 3. Set gb18030 first, gb18030 second, and gb18030 third to 0x00.
            // > 4. If code point is null, return error.
            // > 5. Return a code point whose value is code point.
            return (4, code_point);
        }

        // > 4. If gb18030 second is not 0x00, then:
        if second != 0 {
            // > 1. If byte is in the range 0x81 to 0xFE, inclusive, set gb18030
            // >    third to byte and return continue.
            if let 0x81..=0xFE = byte {
                third = byte;
                continue;
            }
            //> 2. Prepend gb18030 second followed by byte to ioQueue, set
            //>    gb18030 first and gb18030 second to 0x00, and return error.
            return (1, None);
        }

        // > 5. If gb18030 first is not 0x00, then:
        if first != 0 {
            if mode == Mode::Gb2312 && (byte < 0xA1 || byte == 0xFF) {
                // GB2112 is a 94x94 table in G1, so it may use bytes in the
                // region GR_MIN + 1 .. GR_MAX
                return (1, None);
            }

            // > 1. If byte is in the range 0x30 to 0x39, inclusive, set gb18030
            // >    second to byte and return continue.
            if let 0x30..=0x39 = byte {
                second = byte;
                continue;
            }

            // > 2. Let lead be gb18030 first, let pointer be null, and set
            // >    gb18030 first to 0x00.
            let lead = first as u16;

            // > 3. Let offset be 0x40 if byte is less than 0x7F, otherwise
            // >    0x41.
            let offset = if byte < 0x7F { 0x40 } else { 0x41 };

            // > 4. If byte is in the range 0x40 to 0x7E, inclusive, or 0x80 to
            // >    0xFE, inclusive, set pointer to (lead − 0x81) × 190 + (byte
            // >    − offset).
            // > 5. Let code point be null if pointer is null, otherwise the
            // >    index code point for pointer in index gb18030.
            // > 6. If code point is non-null, return a code point whose value
            // >    is code point.
            if let 0x40..=0x7e | 0x80..=0xFE = byte {
                let pointer = (lead - 0x81) * 190 + (byte as u16 - offset);
                if (pointer as usize) < GB18030_INDEX.len() {
                    let code_point = GB18030_INDEX[pointer as usize];
                    return (2, Some(code_point as u32));
                }
            }

            // > 7. If byte is an ASCII byte, prepend byte to ioQueue.
            // > 8. Return error.
            if byte < 0x80 {
                return (1, None);
            }
            return (2, None);
        }

        if mode == Mode::Gb2312 && (byte < 0xA1 || byte == 0xFF) {
            // GB2112 is a 94x94 table in G1, so it may use bytes in the
            // region GR_MIN + 1 .. GR_MAX. ASCII bytes are handled separately
            // by the ISO 2022 encoder
            return (1, None);
        }

        match byte {
            // > 6. If byte is an ASCII byte, return a code point whose value is byte.
            ..=0x7f => return (1, Some(byte as u32)),
            // > 7. If byte is 0x80, return code point U+20AC.
            0x80 => return (1, Some(0x20AC)),
            // > 8. If byte is in the range 0x81 to 0xFE, inclusive, set gb18030
            // >    first to byte and return continue.
            0x81..=0xFE => first = byte,
            // > 9. Return error.
            _ => return (1, None),
        }
    }
    unreachable!()
}

/// Translates Unicode to 1, 2 or 4 bytes output
///
/// Modeled after: https://encoding.spec.whatwg.org/#gb18030-encoder
fn backward_whatwg_gb18030(mode: Mode, out: &mut [u8], code: u32) -> BackwardResult {
    // > 1. If code point is end-of-queue, return finished.
    // > 2. If code point is an ASCII code point, return a byte whose value is code point
    if code < 0x80 {
        if mode == Mode::Gb2312 {
            // ASCII should be handled by the ISO 2022 codec.
            return None;
        } else {
            out[0] = code as u8;
            return Some(1);
        }
    }

    // > 3. If code point is U+E5E5, return error with code point.
    // > NOTE: Index gb18030 maps 0xA3 0xA0 to U+3000 rather than U+E5E5 for
    // > compatibility with deployed content. Therefore it cannot roundtrip.
    if code == 0xE5E5 {
        return None;
    }

    // > 4. If is GBK is true and code point is U+20AC, return byte 0x80.
    if mode == Mode::Gbk && code == 0x20ac {
        out[0] = 0x80;
        return Some(1);
    }

    // > 5. Let pointer be the index pointer for code point in index gb18030.
    if code <= u16::MAX as u32 {
        let searched_code = code as u16;
        // > 6. If pointer is non-null, then:
        if let Some(pointer) = GB18030_INDEX.iter().position(|&c| c == searched_code) {
            // > 1. Let lead be pointer / 190 + 0x81.
            let lead = pointer / 190 + 0x81;
            // > 2. Let trail be pointer % 190.
            let trail = pointer % 190;
            // > 3. Let offset be 0x40 if trail is less than 0x3F, otherwise 0x41.
            let offset = if trail < 0x3F { 0x40 } else { 0x41 };
            // > 4. Return two bytes whose values are lead and trail + offset.
            out[0] = lead as u8;
            out[1] = (trail + offset) as u8;
            return Some(2);
        }
    }

    // > 7. If is GBK is true, return error with code point.
    if mode != Mode::Gb18030 {
        return None;
    }

    // > 8. Set pointer to the index gb18030 ranges pointer for code point.
    let Some(pointer) = backward_whatwg_gb18030_ranges(code)
        else { return None; };

    // > 9. Let byte1 be pointer / (10 × 126 × 10).
    let byte1 = (pointer / (10 * 126 * 10)) as u8;
    // > 10. Set pointer to pointer % (10 × 126 × 10).
    let pointer = pointer % (10 * 126 * 10);
    // > 11. Let byte2 be pointer / (10 × 126).
    let byte2 = (pointer / (10 * 126)) as u8;
    // > 12. Set pointer to pointer % (10 × 126).
    let pointer = pointer % (10 * 126);
    // > 13. Let byte3 be pointer / 10.
    let byte3 = (pointer / 10) as u8;
    // > 14. Let byte4 be pointer % 10.
    let byte4 = (pointer % 10) as u8;
    // > 15. Return four bytes whose values are byte1 + 0x81, byte2 + 0x30, byte3 + 0x81, byte4 + 0x30.
    out[0] = byte1 + 0x81;
    out[1] = byte2 + 0x30;
    out[2] = byte3 + 0x81;
    out[3] = byte4 + 0x30;

    Some(4)
}

/// MultiByteWithoutCodeExtensions GB18030 decoder
pub fn forward_gb18030(c: &[u8]) -> ForwardResult {
    forward_whatwg_gb18030(Mode::Gb18030, c)
}

/// MultiByteWithoutCodeExtensions GB18030 encoder
pub fn backward_gb18030(out: &mut [u8], wc: u32) -> BackwardResult {
    backward_whatwg_gb18030(Mode::Gb18030, out, wc)
}

/// MultiByteWithoutCodeExtensions GBK decoder
pub fn forward_gbk(c: &[u8]) -> ForwardResult {
    forward_whatwg_gb18030(Mode::Gbk, c)
}

/// MultiByteWithoutCodeExtensions GBK encoder
pub fn backward_gbk(out: &mut [u8], wc: u32) -> BackwardResult {
    backward_whatwg_gb18030(Mode::Gbk, out, wc)
}

/// 94x94 table in `G1` `ISO 2022 IR 58` decoder
///
/// Note: GBK is backward compatible with GB2312!
pub fn forward_g1_gb2312(input: &[u8]) -> ForwardResult {
    forward_whatwg_gb18030(Mode::Gb2312, input)
}

/// 94x94 table in `G1` `ISO 2022 IR 58` encoder
///
/// Note: GBK is backward compatible with GB2312!
pub fn backward_g1_gb2312(output: &mut [u8], code: u32) -> BackwardResult {
    backward_whatwg_gb18030(Mode::Gb2312, output, code)
}
