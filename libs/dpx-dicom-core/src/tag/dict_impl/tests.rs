use super::*;
use crate::const_tag_meta_list;

const fn mk_test_meta_array() -> &'static StaticMetaList {
    const_tag_meta_list! {
        const SPECIFIC_CHARACTER_SET = (0x0008, 0x0005), CS, 1, "Specific Character Set", "SpecificCharacterSet", Dicom;
    }
    CONST_META_LIST
}

#[test]
#[rustfmt::skip]
fn is_dict_searchable() {

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
            vr: Vr::LO, alt_vr: Vr::Undefined,
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
fn check_dict_next_field() {
    assert_eq!(Dictionary::dict_next_field("\t").unwrap(), ("", ""));
    assert_eq!(Dictionary::dict_next_field("1\t").unwrap(), ("1", ""));
    assert_eq!(Dictionary::dict_next_field("\t2").unwrap(), ("", "2"));
    assert_eq!(Dictionary::dict_next_field("1\t2").unwrap(), ("1", "2"));

    // This should fail, because no TAB character.
    // Also this test checks if the character position is actual characters, not bytes.
    assert_parser_err!(Dictionary::dict_next_field, "", 0);
    assert_parser_err!(Dictionary::dict_next_field, "Abc", 2);
    assert_parser_err!(Dictionary::dict_next_field, "Абв", 2);
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

    assert_parser_err!(Dictionary::dict_parse_tag_component, "", 0, "expecting hexadecimal");
    assert_parser_err!(Dictionary::dict_parse_tag_component, "T", 0, "invalid character \"T\"");
    assert_parser_err!(Dictionary::dict_parse_tag_component, "012", 3, "expecting hexadecimal");
    assert_parser_err!(Dictionary::dict_parse_tag_component, "01234", 4, "extra characters");
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

    assert_parser_err!(Dictionary::dict_parse_field_tag, "", 0, "expecting opening brace");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "A", 0, "expecting opening brace");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(A", 1, "expecting closing brace");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "()", 1, "expecting comma");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(123456)", 6, "expecting comma");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(123,)", 4, "expecting hexadecimal");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(123456,)", 5, "extra characters");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,)", 6, "expecting hexadecimal");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,)", 11, "no starting or ending");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\")", 11, "no starting or ending");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\")", 12, "empty private creator");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\\\")", 12, "incomplete escape");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\\Z\")", 13, "invalid escape sequence character \"Z\"");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\\u012Z\")", 17, "unable to parse escape");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\r\")", 12, "invalid character");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"\\r\")", 12, "invalid character");
    assert_parser_err!(Dictionary::dict_parse_field_tag, "(1234,6789,\"A\\\\B\")", 13, "invalid character");
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
    assert_eq!(Dictionary::dict_parse_field_vr("UT").unwrap(), (Vr::UT, Vr::Undefined));
    assert_eq!(Dictionary::dict_parse_field_vr("OB or OW").unwrap(), (Vr::OB, Vr::OW));
    assert_eq!(Dictionary::dict_parse_field_vr("  OB  or  OW  ").unwrap(), (Vr::OB, Vr::OW));

    assert_parser_err!(Dictionary::dict_parse_field_vr, "", 0, "unsupported VR");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "ZZ", 0, "unsupported VR");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UTor ", 0, "unsupported VR");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or", 0, "unsupported VR");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or ", 6, "unsupported VR");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or ZZ", 6, "unsupported VR");
    assert_parser_err!(Dictionary::dict_parse_field_vr, "UT or AE or ", 6, "unsupported VR");
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
