#!/bin/bash -e

# This script regenerates all the tags & tags meta files from the corresponding "tsv" files
# Should be run from the workspace root

[[ -e target/generated ]] || mkdir target/generated
cargo r -p mk-tags-rs -- -i libs/dpx-dicom-core/etc/dicom.tsv -a utils/mk-tags-rs/dicom_tags_header.txt -t libs/dpx-dicom-core/src/tags.rs -m libs/dpx-dicom-core/src/tag/dicom_meta.rs
cargo r -p mk-tags-rs -- -i libs/dpx-dicom-core/etc/diconde.tsv -a utils/mk-tags-rs/diconde_tags_header.txt -t libs/dpx-dicom-core/src/tags/diconde.rs -m libs/dpx-dicom-core/src/tag/diconde_meta.rs
cargo r -p mk-tags-rs -- -i libs/dpx-dicom-core/etc/generic.tsv -a utils/mk-tags-rs/generic_tags_header.txt -t libs/dpx-dicom-core/src/tags/generic.rs -m libs/dpx-dicom-core/src/tag/generic_meta.rs
cargo c
