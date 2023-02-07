//! Single-byte encodings supported by the dpx-dicom-encoding crate.

use super::{ForwardResult, BackwardResult};

mod cp_1250;
mod cp_1251;
mod cp_1252;
mod cp_1253;
mod cp_1254;
mod cp_1255;
mod cp_1256;
mod cp_1257;
mod cp_1258;
mod cp_866;
mod iso_ir_100;
mod iso_ir_101;
mod iso_ir_109;
mod iso_ir_110;
mod iso_ir_126;
mod iso_ir_127;
mod iso_ir_13;
mod iso_ir_138;
mod iso_ir_14;
mod iso_ir_144;
mod iso_ir_148;
mod iso_ir_166;
mod iso_ir_203;
mod iso_ir_227;
mod iso_ir_234;
mod iso_ir_6;
mod koi8_r;

pub use cp_1250::*;
pub use cp_1251::*;
pub use cp_1252::*;
pub use cp_1253::*;
pub use cp_1254::*;
pub use cp_1255::*;
pub use cp_1256::*;
pub use cp_1257::*;
pub use cp_1258::*;
pub use cp_866::*;
pub use iso_ir_100::*;
pub use iso_ir_101::*;
pub use iso_ir_109::*;
pub use iso_ir_110::*;
pub use iso_ir_126::*;
pub use iso_ir_127::*;
pub use iso_ir_13::*;
pub use iso_ir_138::*;
pub use iso_ir_14::*;
pub use iso_ir_144::*;
pub use iso_ir_148::*;
pub use iso_ir_166::*;
pub use iso_ir_203::*;
pub use iso_ir_227::*;
pub use iso_ir_234::*;
pub use iso_ir_6::*;
pub use koi8_r::*;

/// Converter function, that always returns an invalid character
pub fn forward_invalid(_: &[u8]) -> ForwardResult {
    (1, None)
}

/// Converter function, that always returns an invalid character
pub fn backward_invalid(_: &mut [u8], _: u32) -> BackwardResult {
    None
}

/// Converter function, that converts text 1-to-1
pub fn forward_identity(input: &[u8]) -> ForwardResult {
    (1, std::char::from_u32(input[0] as u32).map(|c| c as u32))
}

/// Converter function, that always returns an invalid character
pub fn backward_identity(output: &mut [u8], code: u32) -> BackwardResult {
    if code <= 0xFF {
        output[0] = code as u8;
        Some(1)
    } else {
        None
    }
}
