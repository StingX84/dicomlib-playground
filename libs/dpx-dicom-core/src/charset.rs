#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Code {
    Undefined,

    // --------------------------- DEFAULT ------------------------------
    Default,

    // ---------------- Single-Byte Without Code Extensions -------------
    IsoIr100,
    IsoIr101,
    IsoIr109,
    IsoIr110,
    IsoIr144,
    IsoIr127,
    IsoIr126,
    IsoIr138,
    IsoIr148,
    IsoIr203,
    IsoIr13,
    IsoIr166,

    // ----------------- Single-Byte With Code Extensions ---------------
    Iso2022Ir6,
    Iso2022Ir100,
    Iso2022Ir101,
    Iso2022Ir109,
    Iso2022Ir110,
    Iso2022Ir144,
    Iso2022Ir127,
    Iso2022Ir126,
    Iso2022Ir138,
    Iso2022Ir148,
    Iso2022Ir203,
    Iso2022Ir13,
    Iso2022Ir166,

    // ----------------- Multi-Byte With Code Extensions ---------------
    Iso2022Ir87,
    Iso2022Ir159,
    Iso2022Ir149,
    Iso2022Ir58,

    // ----------------- Multi-Byte Without Code Extensions ---------------
    IsoIr192,
    Gb18030,
    Gbk,
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    SingleByteWithoutExtensions,
    SingleByteWithExtensions,
    MultiByteWithExtensions,
    MultiByteWithoutExtensions,
}

#[derive(Debug, Clone)]
pub struct Info {
    pub code: Code,
    pub term: &'static str,
    pub description: &'static str,
    pub kind: Kind,
}

pub const INFO: &'static [Info] = &[
    Info {
        code: Code::Default,
        term: "ISO-IR 6",
        description: "Default repertoire",
        kind: Kind::SingleByteWithoutExtensions,
    },
    // ---------------- Single-Byte Without Code Extensions -------------
    Info {
        code: Code::IsoIr100,
        term: "ISO_IR 100",
        description: "Latin alphabet No. 1",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr101,
        term: "ISO_IR 101",
        description: "Latin alphabet No. 2",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr109,
        term: "ISO_IR 109",
        description: "Latin alphabet No. 3",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr110,
        term: "ISO_IR 110",
        description: "Latin alphabet No. 4",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr144,
        term: "ISO_IR 144",
        description: "Cyrillic",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr127,
        term: "ISO_IR 127",
        description: "Arabic",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr126,
        term: "ISO_IR 126",
        description: "Greek",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr138,
        term: "ISO_IR 138",
        description: "Hebrew",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr148,
        term: "ISO_IR 148",
        description: "Latin alphabet No. 5",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr203,
        term: "ISO_IR 203",
        description: "Latin alphabet No. 9",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr13,
        term: "ISO_IR 13",
        description: "Japanese",
        kind: Kind::SingleByteWithoutExtensions,
    },
    Info {
        code: Code::IsoIr166,
        term: "ISO_IR 166",
        description: "Thai",
        kind: Kind::SingleByteWithoutExtensions,
    },
    // ----------------- Single-Byte With Code Extensions ---------------
    Info {
        code: Code::Iso2022Ir6,
        term: "ISO 2022 IR 6",
        description: "Default repertoire",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir100,
        term: "ISO 2022 IR 100",
        description: "Latin alphabet No. 1",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir101,
        term: "ISO 2022 IR 101",
        description: "Latin alphabet No. 2",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir109,
        term: "ISO 2022 IR 109",
        description: "Latin alphabet No. 3",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir110,
        term: "ISO 2022 IR 110",
        description: "Latin alphabet No. 4",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir144,
        term: "ISO 2022 IR 144",
        description: "Cyrillic",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir127,
        term: "ISO 2022 IR 127",
        description: "Arabic",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir126,
        term: "ISO 2022 IR 126",
        description: "Greek",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir138,
        term: "ISO 2022 IR 138",
        description: "Hebrew",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir148,
        term: "ISO 2022 IR 148",
        description: "Latin alphabet No. 5",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir203,
        term: "ISO 2022 IR 203",
        description: "Latin alphabet No. 9",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir13,
        term: "ISO 2022 IR 13",
        description: "Japanese",
        kind: Kind::SingleByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir166,
        term: "ISO 2022 IR 166",
        description: "Thai",
        kind: Kind::SingleByteWithExtensions,
    },
    // ----------------- Multi-Byte With Code Extensions ---------------
    Info {
        code: Code::Iso2022Ir87,
        term: "ISO 2022 IR 87",
        description: "Japanese",
        kind: Kind::MultiByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir159,
        term: "ISO 2022 IR 159",
        description: "Japanese",
        kind: Kind::MultiByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir149,
        term: "ISO 2022 IR 149",
        description: "Korean",
        kind: Kind::MultiByteWithExtensions,
    },
    Info {
        code: Code::Iso2022Ir58,
        term: "ISO 2022 IR 58",
        description: "Simplified Chinese",
        kind: Kind::MultiByteWithExtensions,
    },
    // ----------------- Multi-Byte Without Code Extensions ---------------
    Info {
        code: Code::IsoIr192,
        term: "ISO_IR 192",
        description: "UTF-8",
        kind: Kind::MultiByteWithoutExtensions,
    },
    Info {
        code: Code::Gb18030,
        term: "GB18030",
        description: "GB18030",
        kind: Kind::MultiByteWithoutExtensions,
    },
    Info {
        code: Code::Gbk,
        term: "GBK",
        description: "GBK",
        kind: Kind::MultiByteWithoutExtensions,
    },
];
