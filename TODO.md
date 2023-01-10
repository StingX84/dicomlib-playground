# Some todo's and "remarks"

Legend:
- Pending: ☐
- Finished: ☑

# Roadmap

## Core library:

### ☑ Value Representation - module "vr"
- Enumeration
- Information accessor

### ☐ Charset encoding - module "charset"
- Configuration parameters specific to charset handling
- "Specific Character Set" attribute parser/validator
- Standard single-paged tables & codec
- ISO 2022 multi-paged tables & codec
- UTF-8 codec
- GB18030 codec
- GBK codec

### ☐ Tag, TagKey and Dictionary - module "tag"
- Basic classes for Tag, TagKey handling
- Define Tag dictionary format
- Dictionary class and static data / file loader

### ☐ Uid and Dictionary - module "uid"
- Basic classes for Uid handling
- Define UID dictionary format
- Dictionary class and static data / file loader

### ☐ Basic I/O implementation - module "io"
- Source/Target traits
- MemorySource class
- BorrowedMemorySource class
- StreamSource class
- FileSource class
- MemoryTarget class
- StreamTarget class
- FileTarget class

### ☐ Data item - module "item"

- Item, Value

### ☐ Data source and parsing classes - module "dataset"
- Data source/Data target trait
- Streamed data source
- Streamed data target
- Borrowed data sources:
  - Memory data source
  - Memory Map data source
- Memory data target
- Dataset

### ☐ Transfer Syntax basic handling and information - module "xfer"
- Basic classes XferSyntax handling
- Trait for PixelData codecs handling
- Codecs for uncompressed pixel data
- Dictionary class and static built-in codecs load


### ☐ Standard parsing utilities
- Define format for Tag dictionary
- Utility to

# Thoughts

## Configuration

### ☐ Implement lib-to-app database fields integration:
Support for the application to pass it's supported DB fields and their characteristics.
Each field should have a unique identifier outside of the DICOM scope. This will allow C-FIND attributes validation, strategy selection and transformation from DICOM to database space and visa-versa.

Field descriptor should provide:
- Unique identifier
- DICOM Attribute TagKey
- Field category
- Field capabilities
- Field type
- A list of children attributes for sequences

Expected field categories:
- Single-valued:
- Multi-valued:

Expected field capabilities:
- Allow "empty" attribute match
- Allow "empty" value match (for Multi-Valued only)
- Allow wildcard match

Expected field types:
- Regular - Any numeric or text type.
- Date - Date-only type. Applicable for `DA` tags. This type prohibits `DA`/`DT` combined range matching and yields poor timezone support.
- Time - Time-only type. Applicable for `TM` tags.
- Timestamp - Timestamp type. Applicable for `DA` and `DT` tags.
- OID - Text containing a DICOM UID. Applicable for `UI` tags.
- Sequence - Some structured data (JSON, joined table, ...). Applicable for `SQ` tags.


### DA/TM association
Library stores associated `DA` and `TM` attributes in the dictionary and knows hows to "combine" them to a single timestamp in incoming C-FIND requests and how to "break" timestamp in outgoing C-FIND requests. When incoming C-FIND "combined" into a single "timestamp" it returns it in `DA` attribute in UTC timezone to the application and "forgets" associated `TM`.

Recommendations for the application:
- Save timezone in DB in one of the ways:
  - For EACH `DA`/`DT` pair store additional field with timezone (16-bit signed would be enough)
  - Store timezone at "Study" levels.
- One of the following:
  - Make an index of `DA`/`TM` combination as timestamp
  - Store timestamp in `DA` field with a time set to a value of corresponding `TM` or midnight, if `TM` empty.
- When creating DB entry from a received Dataset:
  - "combine" received `DA`/`TM` into a single timestamp accounting time-zone attribute and negotiated time-zone.\
    Note: empty `TM` and/or `DA` may be substituted with a current UTC system time.
  - extract and save timezone, convert timestamp to UTC
  - "break" timestamp to `DA`/`TM` again and use these values in DB.
- When constructing C-FIND response or updating an outgoing Dataset with a values from database, application should
  reverse `DA`/`TM` transformation:
  - If dataset does not contain time-zone offset, then set dataset time-zone to a value stored in the database for a first `DA` encountered.
  - convert timestamp to a timezone of the dataset.
  - break timestamp to `DA`/`TM` and put them into the dataset


☐ Implement QR matching methods:\
Note: Standard "Universal Matching" and "Wild Card Matching" are no-op, because always matched.
- Matching to a single-valued DB fields:
  - Match One Empty - Match if a single-valued DB field is empty. Standard name: "Empty Value Matching"
  - Match One To One - Exact match of a single value with a single-valued DB field. Standard name: "Single Value Matching".
  - Match One Glob One - Wild-carded match of single value to a single-valued DB field. Standard name: "Wild Card Matching".
  - Match Any To One - Exact match of any value to a single-valued DB field. Standard name: "List of UID Matching".
  - Match Any Glob One - Wild-carded match of any value to a a single-valued DB field.
  - Match Date In Range - Match if a date-range value matches DB field. St
- Matching to a multi-valued DB fields:

  - Match One To Any - Match a single value with one of the values in a multi-valued DB field. Empty matched field should match with empty DB field or any empty value in DB field. Standard name: "Single Value Matching".
  -
- Match List Empty - Match if a multi-valued DB field is empty (contains no values).
- Match Single In List -
- Match List Any Single - One of the matched values should match single-valued DB field. If one the searched values is empty, it should match to empty single-valued DB field. Empty matched list should match to empty DB field.
- Match List Any  In List - One of the matched values should match to one of the values in multi-valued DB field. If one the searched values is empty, it should match if multi-valued DB field contains empty value.
- Match All Values In List - All of the matched values should be present in multi-valued DB field in any order. DB field may contain more values than searched. If one the searched values is empty, it should match if multi-valued DB field contains empty value.
- Match Any Wildcard - One of the matched values containing wildcard symbols should match to single-valued DB field
- Match Any Wildcard In List - One of the matched values containing wildcard symbols should match to one of value in a multivalued-valued DB field

### ☐ Add configuration for C-FIND request with "by VR" and "by Tag" filters:
- `Multi-valued Attribute`: Standard/Fail/Ignore Attribute/Force "Match All Values"/Force "Match Any Value"\
  Standard:
  - If Extended Negotiation for Multiple Value Matching successful or forced:
    - For `UI` - "Match Any Value" aka standard "List Of UID Matching"
    - For `AE`, `AS`, `AT`, `CS`, `LO`, `PN`, `SH`, or `UC` - "Match Any Value" aka standard "Multiple Value Matching"
    - For others: Ignore attribute
  - If negotiation was not successful or forced:(undefined by Standard, but defined here):
    - "Match Any Value" if DB field
- `Empty value in multi-valued Attribute`: Standard/Fail/Ignore Attribute/Ignore Value/Force "Universal matching" for attribute
- `Literal "" in single-valued Attribute`: Standard/Fail/Ignore Attribute/Force "Empty Value Matching"\
  Standard:
  - If Extended Negotiation of Empty Value Matching is successful or forced:
    - For `AE`, `CS`, `DA`, `DT`, `LO`, `LT`, `PN`, `SH`, `ST`, `TM`, `UC`, `UR`, or `UT` - "Empty Value Matching"
    - For other text-based VR's `AS`, `DS`, `IS`, `UI` - These chars are not allowed, so attribute is ignored
  - If negotiation was not successful or forced:
    - "Single Value Matching"
- `Literal "" in multi-valued Attribute`: Standard/Fail/Ignore Attribute/Ignore Value/Force "Empty Value Matching"\
  Standard:
  - Literal matching in either Multiple Value (all) or List Matching (all)
- `Wildcard in single-valued Attribute`: Standard/Fail/Ignore Attribute
- `Wildcard in multi-valued Attribute`: Standard/Fail/Ignore Attribute/Ignore Value/Force "Wildcard Matching"
- `Literal * in single-valued Attribute`: Standard/Fail/Ignore Attribute
- `Literal * in multi-valued Attribute`: Standard/Fail/Ignore Attribute/Ignore Value/Force "Universal matching" for attribute

☐ Association "auto negotiation" for Query/Retrieve:
- `Negotiate relational-query support`: Allow/Deny/Force
- `Negotiate combined date and time matching`: Allow/Deny/Force
- `Negotiate fuzzy-matching for PN`: Allow/Deny/Force
- `Negotiate timezone query adjustment`: Allow/Deny/Force
- `Negotiate Enhanced Multi-Frame Image Conversion`: Allow/Deny/Force
- `Negotiate Empty Value Matching Support`: Allow/Deny/Force
- `Negotiate Multiple Value Matching Support`: Allow/Deny/Force
