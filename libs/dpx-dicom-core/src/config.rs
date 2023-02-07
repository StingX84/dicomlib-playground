use crate::{
    settings::{Key, KeyMeta, Concept, MaybeGenerated, StaticRegistry, Value, ValueMeta},
};
//use dpx_dicom_charset::{Term, ALL_ENCODINGS};

use crate::vr::Vr;

const THIS_MODULE: &str = "dpx-dicom-core";
const DISPLAY_SECTION_DATASET: &str = "Dataset";

pub const DEFAULT_SPECIFIC_CHARSET: Key = Key::new(THIS_MODULE, line!());
pub const DEFAULT_TIMEZONE_OFFSET: Key = Key::new(THIS_MODULE, line!());
pub const DELIMITER_FOR_TAG: Key = Key::new(THIS_MODULE, line!());
pub const DELIMITER_FOR_VR: Key = Key::new(THIS_MODULE, line!());
pub const FORCE_SPECIFIC_CHARSET: Key = Key::new(THIS_MODULE, line!());
pub const FORCE_TIMEZONE_OFFSET: Key = Key::new(THIS_MODULE, line!());
pub const NON_CONFORMING_TAGS: Key = Key::new(THIS_MODULE, line!());
pub const USE_ALL_SPECIFIC_CHARSET_FOR_PN: Key = Key::new(THIS_MODULE, line!());


pub(crate) enum NonConformingTags {
    Deny,
    Ignore,
    Fix,
}

// fn make_specific_encodings_meta_enum() -> Box<dyn Iterator<Item = (u32, Concept)>> {
//     Box::new(
//         ALL_ENCODINGS
//             .iter()
//             .map(|e| (e.term as u32, Concept::new(e.keyword, e.description, None))),
//     )
// }

inventory::submit! { StaticRegistry(&[
    // KeyMeta {
    //     key: DEFAULT_SPECIFIC_CHARSET,
    //     is_advanced: false,
    //     display_section: DISPLAY_SECTION_DATASET,
    //     concept: Concept::new(
    //         "defaultSpecificCharset",
    //         "Default Specific Character Set",
    //         Some("Sets the default value of Specific Character Set (0008:0005) if not specified in incoming datasets. Also, this value used in outgoing datasets if `Force Specific Character Set` is not defined.")
    //     ),
    //     value_meta: ValueMeta::Enum {
    //         values: MaybeGenerated::Dynamic(make_specific_encodings_meta_enum)
    //     },
    //     make_default: || Some(Value::Enum(Term::IsoIr192 as u32)),
    // },
    // KeyMeta {
    //     key: DEFAULT_TIMEZONE_OFFSET,
    //     is_advanced: false,
    //     display_section: DISPLAY_SECTION_DATASET,
    //     concept: Concept::new(
    //         "defaultTimezoneOffset",
    //         "Default Timezone Offset",
    //         // cSpell:ignore HHMM
    //         Some("Sets the default value of Timezone Offset From UTC (0008:0021) if not specified in incoming and outgoing datasets. Format: `&HHMM`, where `&` - sign `+` or `-`, `HH` - Hours, `MM` - minutes. Minimum value: -1200, Maximum: +1400.")
    //     ),
    //     value_meta: ValueMeta::String { regexp: Some(r"^((((-1[01])|(-0[0-9])|(\+0[0-9])|(\+1[0-3]))[0-5][0-9])|(-1200)|(\+1400))$"), min_length: Some(5), max_length: Some(5) },
    //     make_default: || None,
    // },
    KeyMeta {
        key: DELIMITER_FOR_TAG,
        is_advanced: true,
        display_section: DISPLAY_SECTION_DATASET,
        concept: Concept::new(
            "delimiterForTag",
            "Attribute values delimiter for Tag",
            Some(r#"According to the DICOM standard, the delimiter of the Attribute values should be symbol with a code 0x5C ('\' BACKSLASH). This setting allows to override it for a specific Tag key. Use empty string to totally disable value separation."#),
        ),
        value_meta: ValueMeta::Map {
            keys: &ValueMeta::Vr {
                one_of: Some(MaybeGenerated::Static(&[Vr::AE, Vr::AS, Vr::AT, Vr::CS, Vr::DA, Vr::DS, Vr::DT, Vr::IS, Vr::LO, Vr::PN, Vr::SH, Vr::TM, Vr::UC, Vr::UI, Vr::UT])),
            },
            values: &ValueMeta::String {
                regexp: Some(r"^[\x20-\x7F]|?$"),
                min_length: Some(0),
                max_length: Some(1),
            },
            min: None,
            max: None,
        },
        make_default: || None,
    },
    KeyMeta {
        key: DELIMITER_FOR_VR,
        is_advanced: true,
        display_section: DISPLAY_SECTION_DATASET,
        concept: Concept::new(
            "delimiterForVr",
            "Attribute values delimiter for VR",
            Some(r#"According to the DICOM standard, the delimiter of the Attribute values should be symbol with a code 0x5C ('\' BACKSLASH). This setting allows to override it for a specific Value Representations. Use empty string to totally disable value separation."#),
        ),
        value_meta: ValueMeta::Map {
            keys: &ValueMeta::Vr {
                one_of: None,
            },
            values: &ValueMeta::String {
                regexp: Some(r"^[\x20-\x7F]?$"),
                min_length: Some(0),
                max_length: Some(1),
            },
            min: None,
            max: None,
        },
        make_default: || None,
    },
    // KeyMeta {
    //     key: FORCE_SPECIFIC_CHARSET,
    //     is_advanced: true,
    //     display_section: DISPLAY_SECTION_DATASET,
    //     concept: Concept::new(
    //         "forceSpecificCharset",
    //         "Force Specific Character Set",
    //         Some("This setting overrides Specific Character Set (0008:0005) in incoming datasets.")
    //     ),
    //     value_meta: ValueMeta::Enum {
    //         values: MaybeGenerated::Dynamic(make_specific_encodings_meta_enum)
    //     },
    //     make_default: || None,
    // },
    KeyMeta {
        key: NON_CONFORMING_TAGS,
        is_advanced: false,
        display_section: DISPLAY_SECTION_DATASET,
        concept: Concept::new(
            "nonConformingTags",
            "Allow invalid Attribute values",
            Some("This setting sets the application behavior when it meets an error when reading a Dataset Attribute from a disk or from a Dicom message."),
        ),
        value_meta: ValueMeta::Enum {
            values: MaybeGenerated::Static(&[
                ( NonConformingTags::Deny as u32, Concept::new("deny", "Deny", Some("Complete reading with an error")) ),
                ( NonConformingTags::Ignore as u32, Concept::new("ignore", "Ignore", Some("Ignore the Attribute and consider it empty")) ),
                ( NonConformingTags::Fix as u32, Concept::new("fix", "Fix", Some("Try to fix any problems with the Attribute or find a workaround")) ),
            ])
        },
        make_default: || Some(Value::Enum(NonConformingTags::Deny as u32)),
    },
])}
