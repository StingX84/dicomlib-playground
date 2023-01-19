#![allow(unused_imports)]

use clap::Parser;
use dpx_dicom_core::{tag::*, Vr};
use log::{info, trace};
use snafu::{prelude::*, Whatever};
use std::{
    borrow::Cow,
    env, fs,
    io::Write,
    ops::{Index, IndexMut},
    path::{Path, PathBuf},
    process::ExitCode,
};

type Result<T, E = Whatever> = std::result::Result<T, E>;

// cSpell:ignore metas canonicalize

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Cli {
    /// tsv dictionary file name
    #[arg(
        short = 'i',
        long,
        default_value_os = "libs/dpx-dicom-core/etc/dicom.tsv"
    )]
    input_tsv: PathBuf,

    /// Output file name for Tag constants
    #[arg(short = 't', long, default_value_os = "const_tags.rs")]
    output_tags: PathBuf,

    /// Output file name for tag::Meta constants
    #[arg(short = 'm', long, default_value_os = "const_metas.rs")]
    output_metas: PathBuf,
}

const TAGS_HEADER: &str = include_str!("meta_header.rs");
const META_HEADER: &str = include_str!("meta_header.rs");

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e.to_string());
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "mk_tags_rs=trace");
    }
    pretty_env_logger::try_init().with_whatever_context(|_| "could not initialize logger")?;

    let output_tags_file_name = abs_path(&cli.output_tags)?;
    let output_metas_file_name = abs_path(&cli.output_metas)?;

    info!("Reading tsv file {} ...", cli.input_tsv.to_string_lossy());
    let mut dict = Dictionary::new_empty();
    dict.add_from_file(&cli.input_tsv)
        .with_whatever_context(|e| format!("Unable to parse dictionary: {e}"))?;

    info!(
        "Writing tags to {} ...",
        output_tags_file_name.to_string_lossy()
    );
    write_tags_to_file(dict.iter(), &output_tags_file_name)?;

    info!(
        "Writing metas to {} ...",
        output_metas_file_name.to_string_lossy()
    );
    write_metas_to_file(dict.iter(), &output_metas_file_name)?;

    info!("Done");

    Ok(())
}

fn abs_path<T: AsRef<Path>>(f: T) -> Result<PathBuf> {
    let f = f.as_ref();
    if f.is_relative() {
        let rel_file_path = f
            .parent()
            .with_whatever_context(|| "target file is empty")?;

        let file_path = env::current_dir()
            .with_whatever_context(|e| format!("unable to retrieve current working dir: {e}"))?
            .join(rel_file_path)
            .canonicalize()
            .with_whatever_context(|e| format!("unable to canonicalize output file: {e}"))?;

        let file_name = f
            .file_name()
            .with_whatever_context(|| format!("no output file name provided"))?;

        Ok(file_path.join(file_name))
    } else {
        Ok(f.to_path_buf())
    }
}

fn write_tags_to_file<'a>(tags: impl Iterator<Item = &'a Meta>, file_name: &PathBuf) -> Result<()> {
    let file = fs::File::create(&file_name).with_whatever_context(|e| {
        format!(
            "Unable to open output file \"{}\": {e}",
            file_name.to_string_lossy()
        )
    })?;
    let mut writer = std::io::BufWriter::new(file);

    write_header(&mut writer, TAGS_HEADER).whatever_context("could not write file")?;
    write!(
        &mut writer,
        "\n#![allow(non_upper_case_globals, unused_imports)]\n\
        use crate::{{Tag, TagKey}};\n\
        use std::borrow::Cow;\n\
        // cspell:disable\n\n"
    )
    .whatever_context("could not write file")?;

    for meta in tags {
        writeln!(
            &mut writer,
            "/// {} {} {} {}{}\
            pub const {} : Tag = {};",
            meta.tag_string(),
            meta.vr_string(),
            meta.vm_string(),
            meta.name.escape_default(),
            match &meta.source {
                Source::Retired => " RETIRED!\n#[deprecated(note = \"Retired DICOM tag\")]\n",
                _ => "\n",
            },
            &meta.keyword,
            tag_to_text(&meta.tag),
        )
        .whatever_context("could not write file")?;
    }

    Ok(())
}

#[rustfmt::skip]
fn write_metas_to_file<'a>(
    tags: impl Iterator<Item = &'a Meta>,
    file_name: &PathBuf,
) -> Result<()> {
    let file = fs::File::create(&file_name)
        .whatever_context("could not create file")?;
    let mut writer = std::io::BufWriter::new(file);

    let count = match tags.size_hint() {
        (_, Some(x)) => x,
        _ => whatever!("invalid iterator")
    };

    write_header(&mut writer, META_HEADER).whatever_context("could not write file")?;
    write!(&mut writer, "
#![allow(missing_docs)]
use crate::tag::StaticMetaList;
// cspell:disable

mod _internals {{
    #![allow(unused_imports)]
    use crate::{{ Tag, TagKey, tag::Source, tag::Meta, tag::PrivateIdentificationAction as Pia, Vr }};
    use std::borrow::Cow;
    pub (super) static ALL_TAGS_META: [Meta; {count}] = [\n")
        .whatever_context("could not write file")?;

    for meta in tags {
        use dpx_dicom_core::tag::PrivateIdentificationAction as Pia;
        writeln!(&mut writer,"        Meta {{\
            tag: {}, \
            mask: 0x{:>08X}, \
            vr: (Vr::{},Vr::{},Vr::{}), \
            vm: ({},{},{}), \
            name: Cow::Borrowed(\"{}\"), \
            keyword: Cow::Borrowed(\"{}\"), \
            source: Source::{} \
            }},",
            tag_to_text(&meta.tag),
            meta.mask,
            vr_to_text(meta.vr.0),vr_to_text(meta.vr.1),vr_to_text(meta.vr.2),
            meta.vm.0, meta.vm.1, meta.vm.2,
            meta.name.escape_default(),
            meta.keyword.escape_default(),
            match &meta.source {
                Source::Invalid => "Invalid".to_string(),
                Source::Dicom => "Dicom".to_string(),
                Source::Dicos => "Dicos".to_string(),
                Source::Diconde => "Diconde".to_string(),
                Source::Retired => "Retired".to_string(),
                Source::Vendored(x) => format!("Vendored({}),", match x {
                    Pia::None => "Pia::None",
                    Pia::D => "Pia::D",
                    Pia::Z => "Pia::Z",
                    Pia::X => "Pia::X",
                    Pia::U => "Pia::U",
                })
            },
        ).whatever_context("could not write file")?;
    }

    write!(&mut writer, "    ];
}}

pub static ALL_TAGS_META: StaticMetaList = StaticMetaList::new(&_internals::ALL_TAGS_META);")
        .whatever_context("could not write file")?;

    Ok(())
}

fn write_header(writer: &mut impl Write, header: &str) -> Result<(), std::io::Error> {
    writer.write(
        header
            .replacen("${DATE}", chrono::Local::now().to_rfc2822().as_str(), 1)
            .replacen("${USER}", whoami::username().as_str(), 1)
            .replacen("${HOST}", whoami::hostname().as_str(), 1)
            .replacen(
                "${CMD_LINE}",
                env::args().collect::<Vec<String>>().join(" ").as_str(),
                1,
            )
            .as_bytes(),
    )?;
    Ok(())
}

fn tag_to_text(tag: &Tag) -> String {
    format!(
        "Tag{{ key: TagKey(0x{:>08X}), creator: {}}}",
        tag.key.as_u32(),
        match &tag.creator {
            Some(c) => format!("Some(Cow::Borrowed(\"{}\"))", c.escape_default()),
            None => "None".to_string(),
        },
    )
}

fn vr_to_text(vr: Vr) -> &'static str {
    match vr {
        Vr::Undefined => "Undefined",
        _ => vr.keyword(),
    }
}