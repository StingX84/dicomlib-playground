# Project Coding Rules

## Crate dependency order

```
dpx-dicom-core → dpx-dicom-charset → dpx-dicom-data → dpx-dicom-io
                                                      → dpx-dicom-network → dpx-dicom-wado
                                                      → dpx-dicom-dicomdir
                                                      → dpx-dicom (facade)
```

## Error handling

All public functions return `dpx_dicom_core::error::Result<T>`. No new enum error types in public modules.

| Situation | Tool |
|---|---|
| New error at detection point | `dicom_err!(Kind, "msg")` macro — captures location |
| Enrich existing `DicomError` upstream | `.err_context("msg")` / `.err_context_with(\|\| ...)` — preserves location |
| Convert foreign error (e.g. `io::Error`) | `.to_dicom_err("msg")` / `.to_dicom_err_with(\|\| ...)` — kind auto-mapped via `ToErrorKind` |

Import macros explicitly: `use crate::{dicom_err, dicom_ctx};`

## Knowledge-base IDs

Format: `dpxkb_<module>_<nnnn>`. Entries live in `docs/knowledge-base.md`.

| Prefix | Module |
|---|---|
| `ds` | dataset |
| `cs` | charset |
| `io` | file I/O |
| `net` | network / DIMSE |
| `wado` | WADO-RS / WADO-URI |
| `cfg` | configuration |

## Style

- No comments describing *what* code does. Only *why* when non-obvious.
- No `unwrap()` in library code. No `snafu` in new code.
- `ToErrorKind` impls belong in the crate that introduces the foreign dependency, not in `dpx-dicom-core`.
- Make all code documentation in English.

## Workflow

- Do not rush to implement a change until directly asked by user. Present a brief plan and ask user before implementation.
