use clap::Parser;
use dpx_dicom_core::tag::Source;
use log::{info, trace};
use snafu::{prelude::*, Whatever};
use std::{
    borrow::Cow,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

type Result<T, E = Whatever> = std::result::Result<T, E>;

// cSpell:ignore tbody canonicalize

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Cli {
    /// Path to DICOM standard docbook folder
    docbook_path: PathBuf,

    /// Output file name
    #[arg(short, long, default_value_os = "dicom.tsv")]
    output: PathBuf,
}

const HEADER: &str = include_str!("header.txt");

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
        std::env::set_var("RUST_LOG", "mk_dicom_tsv=trace");
    }
    pretty_env_logger::try_init().with_whatever_context(|_| "could not initialize logger")?;

    let output_file_name = abs_path(&cli.output)?;

    let file_name = &cli.docbook_path.join("part06").join("part06.xml");
    info!("Reading {}...", file_name.to_string_lossy());
    let content = std::fs::read_to_string(file_name).with_whatever_context(|_| {
        format!("couldn't open the file {}", file_name.to_string_lossy())
    })?;
    let xml = roxmltree::Document::parse(&content).with_whatever_context(|_| {
        format!("couldn't parse xml file {}", file_name.to_string_lossy())
    })?;
    let version_06 = extract_version(xml.root_element())
        .with_whatever_context(|| "unable to extract version string")?;

    let mut tags = Vec::<Tag>::new();
    parse_table(&mut tags, xml.root_element(), "table_6-1", None)?;
    parse_table(&mut tags, xml.root_element(), "table_7-1", None)?;
    parse_table(&mut tags, xml.root_element(), "table_8-1", None)?;
    parse_table(&mut tags, xml.root_element(), "table_9-1", None)?;

    let file_name = &cli.docbook_path.join("part07").join("part07.xml");
    info!("Reading {}...", file_name.to_string_lossy());
    let content = std::fs::read_to_string(file_name).with_whatever_context(|_| {
        format!("couldn't open the file {}", file_name.to_string_lossy())
    })?;
    let xml = roxmltree::Document::parse(&content).with_whatever_context(|_| {
        format!("couldn't parse xml file {}", file_name.to_string_lossy())
    })?;

    let version_07 = extract_version(xml.root_element())
        .with_whatever_context(|| "unable to extract version string")?;
    parse_table(
        &mut tags,
        xml.root_element(),
        "table_E.1-1",
        Some(Source::Dicom),
    )?;
    parse_table(
        &mut tags,
        xml.root_element(),
        "table_E.2-1",
        Some(Source::Retired),
    )?;

    info!("Sorting ...");
    tags.sort_by(|l, r| l.tag.cmp(&r.tag));

    info!("Writing {} ...", output_file_name.to_string_lossy());
    let header = HEADER
        .replacen(
            "${VERSION}",
            format!("{version_06} and {version_07}").as_str(),
            1,
        )
        .replacen("${DATE}", chrono::Local::now().to_rfc2822().as_str(), 1)
        .replacen("${USER}", whoami::username().as_str(), 1)
        .replacen("${HOST}", whoami::hostname().as_str(), 1);

    let file = fs::File::create(&output_file_name)
        .with_whatever_context(|e| format!("Unable to open output file({e})"))?;
    let mut writer = std::io::BufWriter::new(file);
    writer
        .write(header.as_bytes())
        .with_whatever_context(|e| format!("Unable to write file({e})"))?;
    for f in tags.iter() {
        use dpx_dicom_core::tag::PrivateIdentificationAction as V;
        let source = match f.source {
            Source::Invalid => "",
            Source::Dicom => "Dicom",
            Source::Dicos => "Dicos",
            Source::Diconde => "Diconde",
            Source::Retired => "ret",
            Source::Vendored(V::None) => "Priv",
            Source::Vendored(V::D) => "Priv(D)",
            Source::Vendored(V::Z) => "Priv(Z)",
            Source::Vendored(V::X) => "Priv(X)",
            Source::Vendored(V::U) => "Priv(U)",
        };
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{}",
            sanitize_string(f.tag),
            sanitize_string(f.vr),
            sanitize_string(f.name),
            sanitize_string(f.keyword),
            sanitize_string(f.vm),
            source
        )
        .with_whatever_context(|e| format!("unable to write output file {e}"))?;
    }
    writer
        .flush()
        .with_whatever_context(|e| format!("unable to write output file {e}"))?;
    drop(writer);

    info!("Verifying ...");
    let mut dict = dpx_dicom_core::tag::Dictionary::new_empty();
    dict.add_from_file(&output_file_name)
        .with_whatever_context(|e| format!("Could not load dictionary({e})"))?;
    let metrics = dict.metrics();
    ensure_whatever!(
        metrics.dynamic_tags == tags.len(),
        "Unexpected dictionary count of tags {}",
        metrics.dynamic_tags
    );

    info!("Done. Total {} tags found.", tags.len());

    Ok(())
}

struct Tag<'a> {
    tag: &'a str,
    name: &'a str,
    keyword: &'a str,
    vr: &'a str,
    vm: &'a str,
    source: Source,
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

fn extract_version<'a, 'input>(root: roxmltree::Node<'a, 'input>) -> Option<&'a str> {
    root.children()
        .find(|c| c.has_tag_name("subtitle"))
        .and_then(|c| c.text())
        .and_then(|s| s.split('-').next())
        .map(|s| s.trim())
}

fn parse_table<'a, 'input>(
    output: &mut Vec<Tag<'a>>,
    root: roxmltree::Node<'a, 'input>,
    id: &'_ str,
    source: Option<Source>,
) -> Result<()> {
    info!("Processing table {id}...");
    let table =
        find_by_id(root, id).with_whatever_context(|| format!("could not find table {id}"))?;

    let tbody = table
        .children()
        .find(|c| c.has_tag_name("tbody"))
        .with_whatever_context(|| format!("could not find tbody of {id}"))?;

    let mut count = 0usize;
    for tr in tbody.children().filter(|c| c.is_element()) {
        let mut children = tr.children().filter(|c| c.is_element());

        let tag = Tag::<'a> {
            tag: children
                .next()
                .map(get_cell_text)
                .whatever_context("could not find tag cell")?,
            name: children
                .next()
                .map(get_cell_text)
                .whatever_context("could not find name cell")?,
            keyword: children
                .next()
                .map(get_cell_text)
                .whatever_context("could not find keyword cell")?,
            vr: parse_field_vr(
                children
                    .next()
                    .map(get_cell_text)
                    .whatever_context("could not find vr cell")?,
            )?,
            vm: parse_field_vm(
                children
                    .next()
                    .map(get_cell_text)
                    .whatever_context("could not find vm cell")?,
            )?,
            source: match source {
                Some(ref x) => x.clone(),
                None => parse_field_source(
                    children
                        .next()
                        .map(get_cell_text)
                        .whatever_context("could not find source cell")?,
                )?,
            },
        };
        if tag.tag.is_empty() || tag.keyword.is_empty() {
            trace!("skipping empty {}", tag.tag);
        } else {
            output.push(tag);
        }
        count = count + 1;
    }
    info!("... found {count} tags");
    Ok(())
}

fn find_by_id<'a, 'input>(
    input: roxmltree::Node<'a, 'input>,
    id: &'_ str,
) -> Option<roxmltree::Node<'a, 'input>> {
    if input.is_element() {
        if let Some(attr) = input.attribute((roxmltree::NS_XML_URI, "id")) {
            if attr == id {
                return Some(input);
            }
        }

        input.children().find_map(|c| find_by_id(c, id))
    } else {
        None
    }
}

fn get_cell_text<'a, 'input>(td: roxmltree::Node<'a, 'input>) -> &'a str {
    let mut child_opt = Some(td);
    while let Some(n) = child_opt {
        if let Some(cn) = n.first_child() {
            if let Some(s) = cn.text().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                return s;
            }
        }
        child_opt = n.first_element_child();
    }
    ""
}

fn parse_field_vr(s: &str) -> Result<&str> {
    if s.starts_with("See") || s.is_empty() {
        Ok("--")
    } else {
        Ok(s)
    }
}

fn parse_field_vm(s: &str) -> Result<&str> {
    Ok(s.splitn(2, " or ").next().unwrap())
}

fn parse_field_source(s: &str) -> Result<Source> {
    if s.starts_with("RET") {
        Ok(Source::Retired)
    } else if s == "DICOS" {
        Ok(Source::Dicos)
    } else if s == "DICONDE" {
        Ok(Source::Diconde)
    } else if s.is_empty() || s.starts_with("See") {
        Ok(Source::Dicom)
    } else {
        whatever!("unrecognized tag source \"{s}\"")
    }
}

fn sanitize_string<'a>(s: &'a str) -> Cow<'a, str> {
    if s.bytes().find(|c| *c >= 0x7F).is_some() {
        Cow::Owned(s.chars().filter(|c| c.is_ascii()).collect())
    } else {
        Cow::Borrowed(s)
    }
}
