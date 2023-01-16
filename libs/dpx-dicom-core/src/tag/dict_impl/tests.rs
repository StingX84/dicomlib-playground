use super::*;

// cSpell:ignore тест

mod keys {
    use crate::declare_tags;
    use inventory::submit;
    declare_tags! {
        /// Test tags dictionary
        pub const TEST_TAG_LIST = [
            SpecificCharacterSet: { (0x0008, 0x0005), CS, 1-n, "Specific Character Set", Dicom },
            EscapeTriplet: { (0x1000, 0x0000) & 0xFFFF000F, US, 3, "Escape Triplet", Retired },
            ZonalMap: { (0x1010, 0x0000) & 0xFFFF0000, US, 1-n, "Zonal Map", Retired },
            OverlayRows: { (0x6000, 0x0010) & 0xFF00FFFF, US, 1, "Overlay Rows", Dicom },
            PixelData: { (0x7FE0, 0x0010), OB or OW, 1, "Pixel Data", Dicom },
            Item: { (0xFFFE, 0xE000), Undefined, 1, "Item", Dicom },
            ItemDelimitationItem: { (0xFFFE, 0xE00D), Undefined, 1, "Item Delimitation Item", Dicom},
            SequenceDelimitationItem: { (0xFFFE, 0xE0DD), Undefined, 1-10 n, "Sequence Delimitation Item", Dicom },
        ];

        pub const TEST_PRIVATE_TAG_LIST = [
            Vendor1_4321_AB: { (0x4321, 0x10AB), AS, 1, "", Vendored(None) },
            Vendor2_4321_AB: { (0x4321, 0x10AB, "vendor2"), AS, 1, "", Vendored(None) },
            Vendor3_4321_AB: { (0x4321, 0x10AB, "vendor3"), AS, 1, "", Vendored(None) },
            Vendor4_4321_AB: { (0x4321, 0x12AB), AS, 1, "", Vendored(None) },
            Vendor5_4321_AB: { (0x4321, 0x13AB), AS, 1, "", Vendored(None) },
            Vendor6_4321_AB: { (0x4321, 0x14AB, "vendor5"), AS, 1, "", Vendored(None) },
        ];
    }

    submit!(TEST_TAG_LIST);
    submit!(TEST_PRIVATE_TAG_LIST);
}

fn search_tags_in_dict(dict: &Dictionary) {
    fn assert_search(d: &Dictionary, searched: &Tag, expected: &Tag) {
        let found = d
            .search_by_tag(searched)
            .unwrap_or_else(|| panic!("tag \"{searched}\" was not found"));
        assert_eq!(found, expected);
    }

    // Standard attributes should always be searchable by it's tag
    for m in keys::TEST_TAG_LIST.value() {
        assert_search(dict, &m.tag, &m.tag);
    }
    // Private attributes should always be searchable by it's tag
    for m in keys::TEST_TAG_LIST.value() {
        assert_search(dict, &m.tag, &m.tag);
    }

    assert_search(dict, &Tag::standard(0x1010, 0x1234), &keys::ZonalMap);

    assert_search(dict, &Tag::standard(0x6001, 0x0010), &keys::OverlayRows);

    // If input has a creator, dict should ignore 0x12AB with no creator and fall back to 0x10AB
    assert_search(
        dict,
        &Tag::private(0x4321, 0x12AB, "vendor2"),
        &keys::Vendor2_4321_AB,
    );

    // If input has no creator, and dict has no creator dict should match 0x12AB
    assert_search(dict, &Tag::standard(0x4321, 0x12AB), &keys::Vendor4_4321_AB);

    // If input has no creator, but dict has one dict should also match
    assert_search(dict, &Tag::standard(0x4321, 0x14AB), &keys::Vendor6_4321_AB);

    // If input has a creator, and dict has no creator, it should match
    assert_search(
        dict,
        &Tag::private(0x4321, 0x12AB, "unknown"),
        &keys::Vendor4_4321_AB,
    );

    // Should not coerce to canonical form if no private creator given
    assert!(dict.search_by_tag(&Tag::standard(0x4321, 0x15AB)).is_none());

    // Should not match 0x14AB because of different creator. Also should not match
    // "canonical" 0x10AB, because "canonical" form requires exact creator match.
    assert!(dict
        .search_by_tag(&Tag::private(0x4321, 0x14AB, "unknown"))
        .is_none());
}

#[test]
fn is_dict_searchable() {
    let mut dict = Dictionary::new_empty();
    dict.add_static_list(&keys::TEST_TAG_LIST);
    dict.add_static_list(&keys::TEST_PRIVATE_TAG_LIST);

    // Search in non-cached dictionary
    search_tags_in_dict(&dict);
}

#[test]
fn is_dict_cache_searchable() {
    let mut dict = Dictionary::new_empty();
    dict.add_static_list(&keys::TEST_TAG_LIST);
    dict.add_static_list(&keys::TEST_PRIVATE_TAG_LIST);

    dict.rebuild_cache();
    search_tags_in_dict(&dict);
}

#[test]
#[cfg_attr(miri, ignore)]
fn is_dict_auto_collects_statics() {
    let dict = Dictionary::new();
    search_tags_in_dict(&dict);
}

#[rustfmt::skip]
macro_rules! assert_parser_err {
    ($fn:path, $text:expr, $expected_pos:expr) => {{
        let text: &str = $text;
        match $fn(text) {
            Err(DictParseErr(pos, _)) => {
                let pos = Dictionary::map_offset_to_char_pos(text, pos);
                assert!(pos == $expected_pos, "pos \"{}\" is not equal to expected \"{}\" in error from \"{}\"", pos, $expected_pos, stringify!($fn));
            }
            _ => assert!(false, "{} expected to fail", stringify!($fn)),
        }
    }};
    ($fn:path, $text:expr, $expected_pos:expr, $expected_msg:expr) => {{
        let text: &str = $text;
        match $fn(text) {
            Err(DictParseErr(pos, msg)) => {
                let pos = Dictionary::map_offset_to_char_pos(text, pos);
                assert!(pos == $expected_pos, "pos \"{}\" is not equal to expected \"{}\" in error from \"{}\"", pos, $expected_pos, stringify!($fn));
                assert!(msg.contains($expected_msg), "message \"{}\" does not contains expected \"{}\" in error from \"{}\"", msg.escape_default(), $expected_msg.escape_default(), stringify!($fn));
            }
            _ => assert!(false, "{} expected to fail", stringify!($fn)),
        }
    }};
}

#[test]
#[rustfmt::skip]
fn check_dict_parse_line_int() {
    assert!(Dictionary::dict_parse_line_int(" # Comment").unwrap().is_none());
    assert!(Dictionary::dict_parse_line_int(" ").unwrap().is_none());
    assert_eq!(Dictionary::dict_parse_line_int("(0010,0020)\tPatient ID\tPatientID\tLO\t1\tdicom").unwrap().unwrap(),
        Meta{
            tag: Tag::standard(0x0010, 0x0020),
            mask: 0xFFFFFFFFu32,
            vr: (Vr::LO, Vr::Undefined, Vr::Undefined),
            vm: (1, 1, 1),
            name: Cow::Borrowed("Patient ID"),
            keyword: Cow::Borrowed("PatientID"),
            source: Source::Dicom
        });
    assert_parser_err!(Dictionary::dict_parse_line_int, "a", 0, "unexpected end of line");
    assert_parser_err!(Dictionary::dict_parse_line_int, "(0010,0020)\tPatient ID\tPatientID\tLO\t1", 36, "unexpected end of line");
    assert_parser_err!(Dictionary::dict_parse_line_int, "(0010,0020)\tPatient ID\tPatientID\tLO\t1\t", 38, "unrecognized Source");
}

#[test]
#[rustfmt::skip]
fn check_dict_parse_take_element() {
    assert_eq!(Dictionary::dict_parse_take_element("\t", "\t"), ("", Some("")));
    assert_eq!(Dictionary::dict_parse_take_element("1\t", "\t"), ("1", Some("")));
    assert_eq!(Dictionary::dict_parse_take_element("\t2", "\t"), ("", Some("2")));
    assert_eq!(Dictionary::dict_parse_take_element("1\t2", "\t"), ("1", Some("2")));
    assert_eq!(Dictionary::dict_parse_take_element("", "\t"), ("", None));
    assert_eq!(Dictionary::dict_parse_take_element("Abc", "\t"), ("Abc", None));
    assert_eq!(Dictionary::dict_parse_take_element("A", " or "), ("A", None));
    assert_eq!(Dictionary::dict_parse_take_element("A or", " or "), ("A or", None));
    assert_eq!(Dictionary::dict_parse_take_element("A or B", " or "), ("A", Some("B")));
    assert_eq!(Dictionary::dict_parse_take_element("A or B or C", " or "), ("A", Some("B or C")));
}

#[test]
#[rustfmt::skip]
fn check_dict_parse_tag_component() {
    assert_eq!(Dictionary::dict_parse_tag_component("0123").unwrap(),
        (0x0123, 0xFFFF));
    assert_eq!(Dictionary::dict_parse_tag_component("  0123  ").unwrap(),
        (0x0123, 0xFFFF));
    assert_eq!(Dictionary::dict_parse_tag_component("cDeF").unwrap(),
        (0xcdef, 0xFFFF));
    assert_eq!(Dictionary::dict_parse_tag_component("AxbC").unwrap(),
        (0xA0BC, 0xF0FF));
    assert_eq!(Dictionary::dict_parse_tag_component("xXxX").unwrap(),
        (0x0000, 0x0000));

    assert_parser_err!(Dictionary::dict_parse_tag_component, "", 0, "unexpected end ");
    assert_parser_err!(Dictionary::dict_parse_tag_component, "T", 0, "invalid character \"T\"");
    assert_parser_err!(Dictionary::dict_parse_tag_component, "012", 3, "unexpected end");
    assert_parser_err!(Dictionary::dict_parse_tag_component, "01234", 4, "unexpected extra");
    assert_parser_err!(Dictionary::dict_parse_tag_component, "012Z", 3, "invalid character \"Z\"");
}

#[test]
#[rustfmt::skip]
fn check_dict_parse_field_tag() {
    assert_eq!(Dictionary::dict_parse_field_tag("(4321,5678,\"creator\")").unwrap(),
        (Tag::private(0x4321, 0x5678, "creator"), 0xFFFFFFFFu32));
    assert_eq!(Dictionary::dict_parse_field_tag("(4321,5678,\"тест\")").unwrap(),
        (Tag::private(0x4321, 0x5678, "тест"), 0xFFFFFFFFu32));
    assert_eq!(Dictionary::dict_parse_field_tag("(cDeF,xXaB)").unwrap(),
        (Tag::standard(0xcdef, 0x00ab), 0xFFFF00FFu32));
    assert_eq!(Dictionary::dict_parse_field_tag("(xxxx,xxxx)").unwrap(),
        (Tag::standard(0x0000, 0x0000), 0x00000000u32));
    assert_eq!(Dictionary::dict_parse_field_tag(" ( 4321 , 5678 , \"creator\" ) ").unwrap(),
        (Tag::private(0x4321, 0x5678, "creator"), 0xFFFFFFFFu32));

    let max_creator = String::from_iter(['Ы'; 64]);
    let long_tag = format!("(4321,5678,\"{max_creator}\")");
    assert_eq!(Dictionary::dict_parse_field_tag(long_tag.as_str()).unwrap(),
        (Tag::private(0x4321, 0x5678, max_creator.as_str()), 0xFFFFFFFFu32));

    assert_parser_err!(Dictionary::dict_parse_field_tag, "", 0, "expecting Tag definition");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "A", 0, "expecting Tag definition");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(A", 0, "expecting Tag definition");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "()", 1, "unexpected end of Tag");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(123456)", 5, "unexpected extra");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(123,)", 4, "unexpected end of Tag");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(123456,)", 5, "unexpected extra");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234)", 4, "expecting comma");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,)", 6, "unexpected end of Tag");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,)", 11, "expecting non-empty private");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\")", 11, "expecting non-empty private");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\")", 11, "expecting non-empty private");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\\\")", 12, "incomplete escape");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\\Z\")", 13, "invalid escape sequence character \"Z\"");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\\u012Z\")", 17, "unable to parse escape");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\r\")", 12, "invalid character");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\\r\")", 12, "invalid character");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"A\\\\B\")", 13, "invalid character");
    //                       Char metrics for reference:  01234567890123456789012
    //                                                    0         1         2
    let overflow_tag = format!("(1234,6789,\"{max_creator}!\")");
    assert_parser_err!(Dictionary::dict_parse_field_tag, overflow_tag.as_str(), 12, "creator is too long");

}

#[test]
#[rustfmt::skip]
fn check_dict_parse_field_name() {
    assert_eq!(Dictionary::dict_parse_field_name("Patient's ID").unwrap(), "Patient's ID");
    assert_eq!(Dictionary::dict_parse_field_name("1").unwrap(), "1");
    let max_name = String::from_iter(['A'; 128]);
    assert_eq!(Dictionary::dict_parse_field_name(max_name.as_str()).unwrap(), max_name);

    assert_parser_err!(Dictionary::dict_parse_field_name, "", 0, "unexpected empty");
    let overflow_name = String::from_iter(['A'; 129]);
    assert_parser_err!(Dictionary::dict_parse_field_name, overflow_name.as_str(), 0, "Name field is too long");
    assert_parser_err!(Dictionary::dict_parse_field_name, "\0", 0, "invalid character \"\\x00\"");
    assert_parser_err!(Dictionary::dict_parse_field_name, "a\tb", 1, "invalid character \"\\t\"");
}

#[test]
#[rustfmt::skip]
fn check_dict_parse_field_keyword() {
    assert_eq!(Dictionary::dict_parse_field_keyword("PatientID").unwrap(), "PatientID");
    assert_eq!(Dictionary::dict_parse_field_keyword("A").unwrap(), "A");
    let max_name = String::from_iter(['A'; 64]);
    assert_eq!(Dictionary::dict_parse_field_keyword(max_name.as_str()).unwrap(), max_name);

    assert_parser_err!(Dictionary::dict_parse_field_keyword, "", 0, "unexpected empty");
    let overflow_name = String::from_iter(['A'; 65]);
    assert_parser_err!(Dictionary::dict_parse_field_keyword, overflow_name.as_str(), 0, "Keyword field is too long");
    assert_parser_err!(Dictionary::dict_parse_field_keyword, "a!b", 1, "invalid character \"!\"");
    assert_parser_err!(Dictionary::dict_parse_field_keyword, "0A", 0, "first character \"0\"");
}

#[test]
#[rustfmt::skip]
fn check_dict_parse_field_vr() {
    assert_eq!(Dictionary::dict_parse_field_vr("UT").unwrap(), (Vr::UT, Vr::Undefined, Vr::Undefined));
    assert_eq!(Dictionary::dict_parse_field_vr("--").unwrap(), (Vr::Undefined, Vr::Undefined, Vr::Undefined));
    assert_eq!(Dictionary::dict_parse_field_vr("OB or OW").unwrap(), (Vr::OB, Vr::OW, Vr::Undefined));
    assert_eq!(Dictionary::dict_parse_field_vr("US or SS or OW").unwrap(), (Vr::US, Vr::SS, Vr::OW));
    assert_eq!(Dictionary::dict_parse_field_vr("  OB  or  OW  ").unwrap(), (Vr::OB, Vr::OW, Vr::Undefined));

    assert_parser_err!(Dictionary::dict_parse_field_vr, "", 0, "empty string");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "ZZ", 0, "unsupported VR \"ZZ\"");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UTor ", 0, "unsupported VR \"UTor\"");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or", 0, "unsupported VR \"UT or\"");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or ", 6, "empty string");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or ZZ", 6, "unsupported VR \"ZZ\"");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or AE or ", 12, "empty string");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or AE or CS or ", 18, "too many VR");
    //                      Char metrics for reference:  0123456789012345678
    //                                                   0         1
}

#[test]
#[rustfmt::skip]
fn check_dict_parse_field_vm() {
    assert_eq!(Dictionary::dict_parse_field_vm("0-1").unwrap(), (0, 1, 1));
    assert_eq!(Dictionary::dict_parse_field_vm("1").unwrap(), (1, 1, 1));
    assert_eq!(Dictionary::dict_parse_field_vm("255").unwrap(), (255, 255, 1));
    assert_eq!(Dictionary::dict_parse_field_vm("10-n").unwrap(), (10, 0, 1));
    assert_eq!(Dictionary::dict_parse_field_vm("10-255").unwrap(), (10, 255, 1));
    assert_eq!(Dictionary::dict_parse_field_vm("8-8n").unwrap(), (8, 0, 8));

    assert_parser_err!(Dictionary::dict_parse_field_vm, "", 0, "invalid VM number");
    assert_parser_err!(Dictionary::dict_parse_field_vm, "-", 0, "invalid VM number");
    assert_parser_err!(Dictionary::dict_parse_field_vm, "-2", 0, "invalid VM number");
    assert_parser_err!(Dictionary::dict_parse_field_vm, "1-", 2, "unexpected end of VM");
    assert_parser_err!(Dictionary::dict_parse_field_vm, "0-0", 2, "zero second");
    assert_parser_err!(Dictionary::dict_parse_field_vm, "0", 0, "zero first");
    assert_parser_err!(Dictionary::dict_parse_field_vm, "1n", 1, "unexpected \"n\"");
    assert_parser_err!(Dictionary::dict_parse_field_vm, "2-1", 0, "second VM number");
    assert_parser_err!(Dictionary::dict_parse_field_vm, "2-1n", 0, "unequal numbers");
}

#[test]
#[rustfmt::skip]
fn check_dict_parse_field_source() {
    assert_eq!(Dictionary::dict_parse_field_source("DiCoM").unwrap(), Source::Dicom);
    assert_eq!(Dictionary::dict_parse_field_source("DiCoS").unwrap(), Source::Dicos);
    assert_eq!(Dictionary::dict_parse_field_source("DiCoNdE").unwrap(), Source::Diconde);
    assert_eq!(Dictionary::dict_parse_field_source("ReT").unwrap(), Source::Retired);
    assert_eq!(Dictionary::dict_parse_field_source("PrIv").unwrap(), Source::Vendored(PrivateIdentificationAction::None));
    assert_eq!(Dictionary::dict_parse_field_source("PrIv(d)").unwrap(), Source::Vendored(PrivateIdentificationAction::D));
    assert_eq!(Dictionary::dict_parse_field_source("PrIv(z)").unwrap(), Source::Vendored(PrivateIdentificationAction::Z));
    assert_eq!(Dictionary::dict_parse_field_source("PrIv(x)").unwrap(), Source::Vendored(PrivateIdentificationAction::X));
    assert_eq!(Dictionary::dict_parse_field_source("PrIv(u)").unwrap(), Source::Vendored(PrivateIdentificationAction::U));

    assert_parser_err!(Dictionary::dict_parse_field_source, "", 0, "unrecognized Source");
    assert_parser_err!(Dictionary::dict_parse_field_source, "priv(t)", 0, "unrecognized Source");
}
