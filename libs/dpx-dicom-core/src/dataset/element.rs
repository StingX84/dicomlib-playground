use super::*;

#[derive(Clone)]
pub enum ElementValue<'a> {
    /// For VR: SS
    I16(i16),
    I16Vec(Cow<'a, [i16]>),
    /// For VR: US
    U16(u16),
    U16Vec(Cow<'a, [u16]>),
    /// For VR: SL
    I32(i32),
    I32Vec(Cow<'a, [u16]>),
    /// For VR: UL, AT
    U32(u32),
    U32Vec(Cow<'a, [u32]>),
    /// For VR: SV
    I64(i64),
    I64Vec(Cow<'a, [i64]>),
    /// For VR: UV
    U64(u64),
    U64Vec(Cow<'a, [u64]>),
    /// For VR: AE, AS, CS, DS, DT, IS, LO, LT, PN, SH, ST, TM, UC, UI, UR, UT
    RawText(Cow<'a, [u8]>),
    /// For VR: FL
    Float(f32),
    FloatVec(Cow<'a, f32>),
    /// For VR: FD
    Double(f64),
    DoubleVec(Cow<'a, f64>),
    /// For VR: SQ
    Dataset(Cow<'a, Container<'a>>),
    DatasetVec(Vec<Container<'a>>),
    /// Any VR, that comes from some sort of stream (lazy load)
    Stream(),
    /// Any VR, that is mapped into the memory. May be borrowed.
    ContiguousMap(Cow<'a, f64>),
    /// Any VR, that is mapped into the memory with "chunks"
    ChunkedMap(Vec<&'a [u8]>),
}

#[derive(Clone)]
enum Inner<'a> {
    Owned{tag: Tag<'static>, value: ElementValue<'static>, context: ElementContext},
    Borrowed{tag: Tag<'a>, value: ElementValue<'a>, context: &'a ElementContext},
}

pub struct Element<'a> { inner: Inner<'a> }

pub struct ElementContext {
    /// Timezone offset from an attribute `Timezone Offset From UTC (0008,0201)` or
    /// from a negotiated `Timezone query adjustment` from the association.
    time_offset_from_utc: u16,
    specific_character_set: Cow<'static, str>,
    xfer: TransferSyntax,
}
