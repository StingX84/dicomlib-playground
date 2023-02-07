# Core Module:

------------------------------------------------------------------------------
## `#dpxkb_ds_0001` - Empty character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` is
present, but empty (contains no values).
### Reason:
Empty values are prohibited by The DICOM Standard (see [PS3.3
C.12.1](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-1)):
the attribute `(0008,0005) Specific Character Set` is listed with type `1C` ,
which requires non-empty value if present.
### Affects:
Character set conversion is disabled to minimize a chance to irreversibly damage
the dataset text attributes in a read-modify-write cycle.


------------------------------------------------------------------------------
## `#dpxkb_ds_0002` - Unknown encoding in character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` contains
an unrecognized `term` in one of it's values.
### Reason:
Only the `term` defined in the DICOM Standard (see [PS3.3
C.12.1.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2))
or some supported non-standard encoding may be used here. The list of
additionally supported encodings are: `cp1250` through `cp1258`, `cp866` and
`koi8-r`.
### Affects:
Character set conversion is disabled to minimize a chance to irreversibly damage
the dataset text attributes in a read-modify-write cycle.


------------------------------------------------------------------------------
## `#dpxkb_ds_0003` - Non standard encoding in character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` contains
a non standard encoding.
### Reason:
Only encodings listed in [PS3.3
C.12.1.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2)
may be used in this attribute. Note, that some non-standard encodings may be allowed by the
application configuration.
### Affects:
Character set conversion is disabled to minimize a chance to irreversibly damage
the dataset text attributes in a read-modify-write cycle.


------------------------------------------------------------------------------
## `#dpxkb_ds_0004` - Non standard encoding accepted in character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` contains
a non standard encoding permitted by the application configuration.
### Reason:
Usage of this encoding violates the DICOM standard (see [PS3.3
C.12.1.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2))
and may cause irreversible data loss when transferring dataset across different
applications.
### Affects:
Text encoding and decoding operations will use this non-standard encoding.


------------------------------------------------------------------------------
## `#dpxkb_ds_0005` - Encoding in the multi-valued character set string does not support ISO-2022 extensions
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` is
multi-valued and one of it's values specifies an encoding not supporting
ISO-2022 extensions.
### Reason:
Multi-valued Specific Character Set attribute may be used exclusively in context
of ISO-2022 text codec to specify available code pages (`Terms`) (ISO-IR
registered 94, 96 or 94x94 sets). For example: `ISO 2022 IR 6`, `ISO 2022 IR 149`.
These `Terms` are listed in the DICOM Standard tables [PS3.3
Table C.12-2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-2)
and [PS3.3
Table C.12-5](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-5).

Any other `Term` such as `ISO_IR 6`, `ISO_IR 192`, `GB18030` could not be used in a multi-valued
`(0008,0005) Specific Character Set` attribute.

Standard citation on single-byte encodings:
> Defined Terms for the Attribute Specific Character Set (0008,0005), ___when
> single valued___, are derived from the International Registration Number as
> per ISO 2375 (e.g., ISO_IR 100 for Latin alphabet No. 1). See [PS3.3
> Table C.12-2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-2).

Standard citation on multi-byte encodings:
> The following multi-byte character sets prohibit the use of Code Extension
> Techniques:
> - The Unicode character set used in [ISO/IEC 10646], when encoded in UTF
> - The [GB 18030] character set, when encoded per the rules of [GB 18030]
> - The [GBK] character set encoded per the rules of [GBK]
>
> These character sets may only be specified as value 1 in the Specific
> Character Set (0008,0005) Attribute and there ___shall only be one value___.

### Affects:
Character set conversion is disabled to minimize a chance to irreversibly damage
the dataset text attributes in a read-modify-write cycle.


------------------------------------------------------------------------------
## `#dpxkb_ds_0006` - First encoding is Multi-Byte in the multi-valued character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` is
multi-valued and it's first value specifies some multi-byte encoding, that could
be used only in values other than the first.
### Reason:
First value of multi-values character set must be one of single-byte character
sets listed in the DICOM [PS3.3
Table C.12-2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-2).
The reasoning behind this is very solid: "Multi-Byte Character Sets with Code
Extensions" encodings (94x94 ISO_IR tables), does not support control
characters, such as `CR`, `LF`. But most importantly, that `space` is not
supported either, making it impossible to "pad" a text field to an even length.
That is why only single-byte tables (such as `ISO 2022 IR 6`) are allowed in the
first value.

Standard citation on this topic:
> Table [C.12-3](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-3)
> describes single-byte character sets for value 1 to value n of the Attribute
> Specific Character Set (0008,0005), and
> Table [C.12-4](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-4)
> describes multi-byte character sets for ___value 2 to value n___ of the Attribute
> Specific Character Set (0008,0005).
### Affects:
Character set conversion is disabled to minimize a chance to irreversibly damage
the dataset text attributes in a read-modify-write cycle.


------------------------------------------------------------------------------
## `#dpxkb_ds_0007` - Non standard encoding aliased name accepted in character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` contains
non-standard alias name for the standard encoding.
### Reason:
Aliased names for DICOM-defined encodings is a non-portable application
extension, which can be disabled in the configuration. For example, `ISO-8859-1`
is an alias to `ISO_IR 100`. Standard terms written in non-uppercase or with
extra/missing spacing, such as `iso_ir 100`, `ISO IR 100` or `IsoIr100` is
supported and also considered as an alias. This extension violates the DICOM
Standard (see [PS3.3
C.12.1.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2)),
but should be safe to accept to maximize interoperability with "buggy" software.
### Affects:
Encoding processed as if standard name were used. When a dataset being written
back to the disk, it's Specific Character Set attribute will be overwritten with
a Standard conforming name.


------------------------------------------------------------------------------
## `#dpxkb_ds_0008` - Ignored empty value in multi-valued specific character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` is
multi-valued and some value other than the first is empty.
### Reason:
According to The DICOM Standard, *Only the first* value may be empty and others
must not. The application is configured to relax this restriction and ignore
empty values. This relaxation is only valid if the attribute still remains
multi-valued after value ignorance.

Citation from [PS3.3 C.12.1.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2):
> Table C.12-4 describes multi-byte character sets for value 2 to value n of the
> Attribute Specific Character Set (0008,0005)

Later in the same section:
> If the Attribute Specific Character Set (0008,0005) has more than one value
> and value 1 is empty, it is assumed that value 1 is ISO 2022 IR 6.

### Affects:
Encoding is processed as if there were no empty value defined in it. This may
lead to the irreversible text corruption, because non-standard Specific
Character Set attribute is not a good sign in the first place.


------------------------------------------------------------------------------
## `#dpxkb_ds_0009` - Ignored duplicate value in multi-valued specific character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` is
multi-valued and contains a duplicate value.
### Reason:
Duplicate values are explicitly prohibited by the Dicom Standard:

Citation from [PS3.3 C.12.1.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2):
> The same character set shall not be used more than once in Specific Character
> Set (0008,0005).

The application is configured to bypass this limitation by ignoring a duplicate
value in cases when Specific Character Set still remain multi-valued.

### Affects:
Encoding is processed as if there were no duplicate value defined in it. This may
lead to the irreversible text corruption, because non-standard Specific
Character Set attribute is not a good sign in the first place.


------------------------------------------------------------------------------
## `#dpxkb_ds_0010` - Empty value in multi-valued specific character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` is
multi-valued and some value other than the first is empty.
### Reason:
Citation from [PS3.3 C.12.1.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2):
> Table C.12-4 describes multi-byte character sets for value 2 to value n of the
> Attribute Specific Character Set (0008,0005)

Later in the same section:
> If the Attribute Specific Character Set (0008,0005) has more than one value
> and value 1 is empty, it is assumed that value 1 is ISO 2022 IR 6.

It is clear, that *only the first* value may be empty and others must not. Such
empty values may be ignored if enabled in the configuration and this action
will not render character set single-valued.

### Affects:
Character set conversion is disabled to minimize a chance to irreversibly damage
the dataset text attributes in a read-modify-write cycle.


------------------------------------------------------------------------------
## `#dpxkb_ds_0011` - Duplicate value in multi-valued specific character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` is
multi-valued and contains a duplicate value.
### Reason:
Duplicate values are explicitly prohibited by the Dicom Standard:

Citation from [PS3.3 C.12.1.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2):
> The same character set shall not be used more than once in Specific Character
> Set (0008,0005).

### Affects:
Character set conversion is disabled to minimize a chance to irreversibly damage
the dataset text attributes in a read-modify-write cycle.


------------------------------------------------------------------------------
## `#dpxkb_ds_0012` - Promoted SingleByteWithoutExtensions to SingleByteWithExtensions in multi valued character set
### Category:
`Warning`
### When:
Reading a dataset whose attribute `(0008,0005) Specific Character Set` is
multi-valued and some encoding being Single Byte Without Extensions converted to a corresponding Single Byte With Extensions.
### Reason:
In multi-valued Specific Character Set attribute standard allows only encoding
names starting with `ISO 2022 IR`. They are listed in tables [PS3.3
Table C.12-3](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-3)
and [PS3.3
Table C.12-4](https://dicom.nema.org/medical/dicom/current/output/html/part03.html#table_C.12-4).
But the application configuration allows to deviate from the standard by
treating encodings starting with `ISO_IR` as `ISO 2022 IR` counterparts.

### Affects:
Encoding is processed as if there were Single Byte With Extensions encoding
provided. This may lead to the irreversible text corruption, because
non-standard Specific Character Set attribute is not a good sign in the first
place.
