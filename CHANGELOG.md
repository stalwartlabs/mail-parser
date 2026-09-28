# Change Log

All notable changes to this project will be documented in this file. This project adheres to [Semantic Versioning](https://semver.org/).

## [1.0.0] - Unreleased

A rewrite of the parser for performance, with a new read-only API. Breaking changes are listed under Changed and Removed; the table at the end of this entry maps the 0.11 API to 1.0.

### Added
- Bodies are decoded on access: `MessagePart::decoded()`, `decode_into()`, `decoded_len()` (measures base64 and quoted-printable without decoding), `text()`, `text_into()` and `text_prefix()` (decodes only what the first characters need).
- `MessagePart::decoded_checked()` and `text_checked()` also return the `DecodeProblems` found while decoding (malformed base64 or quoted-printable, unknown transfer encoding, unknown charset, sequences not valid in the charset), which is the RFC 8621 `isEncodingProblem` flag.
- `MessageRef`, a view of the root message or of a nested message, with `id()`, `root_part()`, `container()`, `source()`, `source_bytes()`, `raw()`, `parts()`, the body lists and the header getters; `Message::root()` and `Message::messages()`.
- Access to MIME boundaries (#127): `MessagePart::boundary()`, `MessagePart::delimiter()` and `Message::part_by_boundary()`, and the raw bytes of a part with `raw()`, `raw_headers()` and `raw_body()`, which exclude the line break before the next delimiter as RFC 2046 defines the body part.
- Part roles (#107): `MessagePart::role()` (`PartRole`) gives every part exactly one role, and `MessageRef::other_parts()` returns the parts in no list.
- `MessageRef::has_attachments()`, the RFC 8621 `hasAttachment` rule: an attachment without `Content-Disposition: inline`.
- `MessagePart::html_utf8()` and `strip_charset_meta()` (#109): the HTML with its `<meta>` charset declarations rewritten to UTF-8, looking only before `<body`. The default decoding does not scan HTML.
- `MessagePart::id()`, `message()`, `parent()`, `kind()` (`PartKind`), `children()`, `nested()`, `flags()` (`PartFlags`), `has_problems()`, `in_text_body()`, `in_html_body()`, `is_attachment()`, `is_inline()`, `encoding()` and `content_transfer_encoding()`.
- `PartFlags` records damaged structure: `MISSING_DELIMITER`, `FALLBACK_DELIMITER`, `NO_BLANK_LINE`, `UNTERMINATED`, `NESTING_LIMIT` and `LIMIT_REACHED`.
- `Headers`, a view of the header fields of any part: `iter()`, `get()`, `all()`, `contains()`, `value()`, `has_known()`, `len()` and the typed getters. `Header::raw_name()` keeps the spelling of the name found in the message, `Header::raw_value()` returns the bytes after the colon, and `Header::parse_as()` parses a field again in another form.
- `HeaderForm::parse()` parses a header value on its own into a `ParsedValue`. `HeaderForm` gains `CommaList`, `ContentType`, `Received` and `Ignore`.
- `AddressList` (#98): `iter()` yields `Address::Mailbox` and `Address::Group` items in document order, `mailboxes()` flattens the groups, `groups()` gives the JMAP `asGroupedAddresses` shape, and `first()`, `last()`, `contains()` and `has_groups()`. `TextList` holds message identifiers and comma-separated lists.
- `MessageParser::parse_owned()` gives a `Message<'static>` that owns its input, and `MessageParser::parse_with()` with `MessageBuffers` and `Message::into_buffers()` reuses the storage of one message for the next. `MessageBuffers::shrink_to(max_bytes)` frees that storage when it exceeds `max_bytes`; otherwise the buffers keep the capacity of the largest message parsed with them.
- `MessageParser::header()` sets the form of one header field and `unknown_headers()` the form of unknown fields; `max_depth()`, `max_parts()` and `max_encoded_nesting()` set the limits (64, 10,000 and 3 by default). After the part limit, the rest of the input stays in the part flagged `LIMIT_REACHED`: the last part when it is a leaf of the innermost open multipart, else that multipart, or the `message/rfc822` part whose nested message had no part yet; the containers left open are not flagged `UNTERMINATED`.
- `HeaderName::key()` resolves a name once into a `HeaderKey`, for `Headers::all_key()` and `Headers::get_key()`, which skip the name lookup that `all()` and `get()` do on every call.
- 112 header names from Stalwart's metadata list are `HeaderName` variants: `X-Mailer`, `User-Agent`, `Precedence`, `X-Priority`, the `X-MS-Exchange`, `X-Google` and `X-GitHub` fields and others.
- `Charset`, with `from_label()`, `decode()`, `decode_owned()` and `decode_append()`, and `decoders::charsets::decode()` and `decode_append()`.
- `PartId` and `MessageId`, and `From<Encoding> for u8`.
- Value equality for `HeaderValue`, `TextList`, `AddressList`, `ContentType`, `Received` and `ParsedValue`; `Debug` output that shows the content of the views; `Debug` for the mailbox iterators. `Message`, `MessageParser` and the views are `Send + Sync`, and `Message<'x>` is covariant in `'x`.

### Changed
- The parser was rewritten for performance: SIMD kernels find boundaries and header fields (NEON, AVX2 and SSE2, `memchr` on other targets), the header field parsers work on bounded slices, a message is stored in a handful of flat vectors whatever its size, and bodies are decoded only on access, through the SIMD codecs of encodify. See the README for measurements.
- New dependencies: `memchr`, `simdutf8` and `encodify`, next to `hashify`; `encoding_rs` and `serde` stay optional. `unsafe` code is used in the SIMD kernels and in the audited UTF-8 module, whose two functions turn bytes already validated as UTF-8 into `str` and `String`: string spans validated during the parse, and decoded text bodies validated with simdutf8; the rest of the crate denies it. The minimum supported Rust version is 1.98.
- A parsed message is read-only and is read through `Copy` views (`MessageRef`, `MessagePart`, `Headers`, `Header`, `HeaderValue`, `AddressList`, `ContentType`, `Received` and others) that borrow the message, so a value can no longer outlive the `Message` it came from. `Message` is still `Send`, `Sync`, `Clone` and `Default`; `Message::default()` is an empty message with one root part (id 0, no header field, empty body), so `root_part()`, `parts()` and `messages()` agree.
- The header getters return views by value: `from()`, `to()`, `cc()`, `bcc()`, `reply_to()`, `sender()`, the `resent_*()` address getters and the `list_*()` getters return `Option<AddressList>`; `date()` and `resent_date()` return `Option<DateTime>`; `comments()` and `mime_version()` return `Option<&str>`; `in_reply_to()`, `references()`, `keywords()`, `resent_message_id()` and `return_path()` return `Option<TextList>`; `received()` returns `Option<Received>`.
- `text_bodies()` and `html_bodies()` yield the text of the parts of the text and HTML bodies (#73), converting HTML to text and text to HTML and skipping parts that are not text; 0.11 yielded the parts, which are now `text_body()` and `html_body()`. This change is silent where the item is only measured: `message.text_bodies().map(|body| body.len())` still compiles and now measures text.
- Part identifiers are global: parts are numbered in document order across the root message and every nested message, and `Message::part()` takes such an id. A nested message is a `MessageRef` (`PartKind::Message`, `MessagePart::nested()`), no longer a `Message` inside the part. A base64 or quoted-printable encoded `message/rfc822` part is decoded and parsed during the parse; the offsets of its parts refer to its decoded body, as `MessageRef::source()` tells.
- Address fields (#98) are an `AddressList` of mailboxes and groups in document order, whether or not they contain a group. 0.11 returned `Address::List` when there was no group and turned every run of mailboxes into an unnamed group as soon as there was one; that shape is now `AddressList::groups()`.
- Message identifier fields, Keywords and Content-Language are always a `TextList`, even with one item; 0.11 switched between `Text` and `TextList`. `HeaderValue::as_text()` returns the last item of a list.
- The body lists follow the RFC 8621 `parseStructure` algorithm exactly (#67). An image, audio or video part shown in both bodies is no longer also an attachment, and a named `text/html` alternative that follows a `text/plain` becomes an attachment while the `text/plain` is copied into `html_body`. Where the algorithm is silent: a text part of an alternative whose list an ancestor dropped goes to no list, a multipart that cannot be opened stays an attachment, a part without a usable Content-Type is `text/plain` (`message/rfc822` in a digest), and the name test uses the Content-Disposition `filename`, else the Content-Type `name`, an empty name counting as none. The `multipart/alternative` fix-up also runs when the alternative is not closed.
- `PartKind::InlineBinary` and `MessagePart::is_inline()` follow the Content-Disposition (#70): `inline`, or no disposition on a child of `multipart/related` other than the first. 0.11 derived them from the RFC 8621 list rule, so images referenced by `cid:` in a `multipart/related` were `Binary`. The body lists are not affected.
- Decoding no longer changes the part tree. 0.11 turned a part whose body failed to decode into a text part flagged `is_encoding_problem`; 1.0 keeps its kind, decodes it on a best-effort basis and reports the problems through `decoded_checked()` and `text_checked()`. `MessagePart::encoding()` is the declared Content-Transfer-Encoding, comments allowed (`base64 (comment)` is base64).
- `is_encoding_problem` is split: `MessagePart::has_problems()` and `flags()` for damaged structure, `decoded_checked()` and `text_checked()` for decoding.
- MIME delimiters (#156): a delimiter is `--boundary` at the start of a line followed by `--`, a space, a tab, CR, LF or the end of the input, checked against every open boundary, innermost first. A `--boundary` in the middle of a line ends a part only when the part has no valid delimiter at all, and the part is then flagged `FALLBACK_DELIMITER`. A multipart whose first delimiter is its close delimiter looks for a delimiter in the middle of a line of its preamble the same way (flagged `FALLBACK_DELIMITER`); a multipart closed before any part is flagged `MISSING_DELIMITER` and keeps its content in its preamble.
- Malformed structure: a header block ends at a delimiter line (flagged `NO_BLANK_LINE`), an inner multipart that is never closed is closed by an outer delimiter, a part whose delimiter is missing is decoded and classified by its type, and every part ends by one rule (the end of the input, or before the line break that precedes the next delimiter), so a part never ends past its container: when a delimiter cuts a header block, or ends a part together with its container, the part ends where the container ends, and the last header field may end one line break past `offset_body()`.
- RFC 2047 encoded words are decoded by encodify. No space is inserted between plain text and an adjacent encoded word, whitespace between two encoded words is dropped and whitespace between text and a word is kept as written. Adjacent words of one charset are joined before conversion, so a character split across two words decodes, and one-byte charset names are accepted.
- Base64 bodies: bytes outside the alphabet are skipped and reported, a final unpadded quantum is decoded (0.11 dropped it) and a lone sextet before `=` no longer produces a byte. Quoted-printable bodies: `==` and other invalid escapes are kept as written (0.11 failed the whole part on `==`), trailing blanks at the end of the input are deleted (RFC 2045, section 6.7), and a soft line break right before a delimiter no longer deletes content.
- Header blocks: a line without a colon at the end of the input is no longer a header field, the last field keeps its value when the input has no final line break, and a bare CR in unstructured text becomes a space.
- Dates: the zone reader no longer consumes the next line after a `UT` or `Z` zone, and a one-digit zone such as `+5` means +05:00. Received fields: two-digit years follow the date parser (00 to 49 are 20xx), and `]` ends an address literal inside comments.
- `MessageParser::header(name, form)` replaces the builder shorthands and changes the form of one field without dropping the default forms of the others. `unknown_headers()` also applies to the header names added in 1.0, unless they are registered with `header()`.
- `HeaderName`: the names added in 1.0 are variants (`HeaderName::XMailer`), and an `Other` spelled like a known name, such as `Other("X-Mailer")`, equals, hashes and orders as that name; `MessageParser::header()`, `Headers::get()` and `Headers::all()` treat it as that name. `HeaderName::parse()` accepts every character RFC 5322 allows in a field name (printable US-ASCII except the colon; 0.11 accepted only letters, digits, `-` and `_`) and never allocates, and the `From` conversions keep any other string in `Other` with its spelling (0.11 returned `Other("")`, which matched no field). `Header::name()` returns a `HeaderName` (0.11: `&str`).
- A header field that is present but blank and parsed as `Raw` or `Text` has the value `HeaderValue::Text("")`, so an empty `Subject:` gives `subject() == Some("")`, as RFC 8621 `asText` does; 0.11 returned no value. `HeaderValue::Empty` now means that the value does not parse in its form (an empty address list, date or Content-Type included) or that the field is parsed with `HeaderForm::Ignore`.
- Header field names are matched strictly: a name with whitespace or a control character inside it (`Fr om`, `Sub\tject`) or starting with a colon (`:From`) is an unknown field kept with its spelling, where 0.11 read it as the known field, so mail-parser sees the same From and Subject as DKIM and DMARC verifiers. Space and tab between the name and the colon are still accepted (RFC 5322 obsolete syntax).
- RFC 2231 parameters: the sections of a continued parameter are joined before they are converted, with the charset of the first section that declares one (section 0 in conforming input), and a language parameter (`name-language`) is reported once; 0.11 moved a repeated `'lang'` prefix into the value. A Content-Type or Content-Disposition field keeps at most 1,000 parameters.
- `MessageParser::parse_headers()` records the real body offset; 0.11 reported the end of the input.
- `MessageParser::parse()` returns `None` for input with no header field and no blank line, where 0.11 returned a message without headers and with an empty body, dropping the text. It also returns `None` for empty input and for input of 4 GiB or more.
- `preview_text()`, `preview_html()`, `truncate_text()` and `truncate_html()` take `&str`, remove carriage returns themselves and borrow when they can; `body_preview()` decodes only the beginning of the body.
- `text_to_html()` also escapes `&` and `>`.
- `Charset::from_label()` maps the UTF-8 labels to `Charset::Utf8`; `charset_decoder()`, kept for compatibility, still returns `None` for them.
- With the `serde` feature, a `Message` serializes to a new JSON shape (messages and parts in flat lists, with roles, flags and decoding problems) and implements `Serialize` only.
- `mailbox::mbox::MessageIterator` finds separator lines with `memchr` and copies message contents in bulk. After an I/O error inside a line, the partial line is kept and reading continues on the next call.

### Removed
- The `rkyv` and `base64_slice` features.
- The public fields and struct literals of `Message`, `MessagePart`, `Header`, `ContentType`, `Addr`, `Group` and `Attribute`, the methods that changed them (`Message::remove_header()`, `ContentType::remove_attribute()`), and `into_owned()` on parts and values.
- `PartType` (replaced by `PartKind` and the decoding methods), `Addr` (replaced by `Mailbox`), `Attribute`, the `Address::List` and `Address::Group` shapes, `MessagePartId` (now `PartId`), the `GetHeader` and `MimeHeaders` traits (their methods are inherent), `MessageStream`, and the `core` and `parsers` modules.
- `HeaderValue::unwrap_*()`, `into_*()`, `into_owned()` and `len()`; `HeaderForm::GroupedAddresses` and `HeaderForm::URLs` (use `HeaderForm::Addresses`); `HeaderName::len()`, `is_empty()` and `into_string()`.
- The base64, quoted-printable and hex helpers: `base64_decode()`, `base64_decode_slice()`, `base64_decode_stream()`, `quoted_printable_decode()`, `quoted_printable_decode_char()`, `decode_hex()`, `HEX_MAP` and `BASE64_MAP`, with the `decoders::base64`, `decoders::quoted_printable`, `decoders::hex` and `decoders::encoded_word` modules; use encodify directly (`encodify::base64`, `qp`, `hex` and `rfc2047`). The `decoders::charsets::{map, multi_byte, single_byte, utf}` modules are replaced by `Charset`.
- `MessagePart::is_text_html()`, `is_binary()` and `is_empty()`.
- `PartialEq` for `Message` and `MessagePart`, and `Deserialize` for `Message`.
- `trim_trailing_fwd()`, an internal step of `thread_name()`.

### Fixed
- MIME boundaries were matched anywhere in a line, which silently truncated body parts (#156).
- Inline images in `multipart/related` were reported as `Binary` (#70).
- `attachments` included inline media shown in both body lists, against RFC 8621 (#67).
- `resent_cc()` returned the Resent-To field.
- Registering one header parser disabled the default parsers of every other field.
- An RFC 2231 value with a third apostrophe (`filename*=utf-8''O'Brien.txt`) was split at it.
- The comment of an empty address group was attached to the next group.
- `add_html_token()` panicked on invalid UTF-8.
- `FolderIterator::next()` panicked when called after the end of the folders.
- A multipart whose first delimiter line carried text after the boundary (`--b5Content-Type: ...`) made 0.11 panic in debug builds ("Invalid part ID, could not find multipart"); 1.0 keeps the content as a part.
- RFC 2231 parameter sections were converted one by one, so a character split across two sections, or any section after the first in a charset other than UTF-8, became U+FFFD (`name*0*=utf-8''%E2%98; name*1*=%95.txt` now gives `☕.txt`).
- RFC 2231 continuations took quadratic time in the number of sections or parameters of a field.
- UTF-7 dropped the character that ends a base64 run (`+AKM.` gave `£` instead of `£.`) and did not decode `+-` to `+` (RFC 2152).
- A UTF-16 text with an odd number of bytes lost its last byte silently; it now ends with U+FFFD, as documented.
- `DateTime::to_timezone()` overflowed (a panic in debug builds) for offsets near `i64::MIN` or `i64::MAX`; it saturates.

### Migrating from 0.11

| 0.11 | 1.0 |
|---|---|
| `MessageParser::new().with_mime_headers()`, `with_date_headers()`, `with_address_headers()`, `with_message_ids()`, `with_minimal_headers()` | `MessageParser::new()`: the default forms parse these fields |
| `header_text(name)`, `header_date(name)`, `header_address(name)`, `header_id(name)`, `header_comma_separated(name)`, `header_content_type(name)`, `header_received(name)`, `header_raw(name)`, `ignore_header(name)` | `header(name, form)` with `HeaderForm::Text`, `Date`, `Addresses`, `MessageIds`, `CommaList`, `ContentType`, `Received`, `Raw` or `Ignore` |
| `without_header(name)` | `header(name, HeaderForm::Raw)` |
| `default_header_text()`, `default_header_raw()`, `default_header_ignore()` | `unknown_headers(HeaderForm::Text)`, `Raw` or `Ignore` |
| `MessageStream::new(value).parse_address()` and the other `parse_*()` | `HeaderForm::Addresses.parse(value).value()` and the other forms |
| `message.header_as(name, form)` | `header.parse_as(form)` for each field of `message.headers().all(name)` |
| `message.parts`, `message.parts[i]` | `message.parts()`, `message.part(id)` (ids are global across nested messages) |
| `message.text_body`, `html_body`, `attachments` (lists of ids) | `message.text_body()`, `html_body()`, `attachments()` (iterators of `MessagePart`; `part.id()` gives the id) |
| `message.text_part(i)`, `html_part(i)`, `attachment(i)` | `message.text_body().nth(i)`, `html_body().nth(i)`, `attachments().nth(i)` |
| `message.text_body_count()`, `html_body_count()`, `attachment_count()` | `message.text_body().len()`, `html_body().len()`, `attachments().len()` |
| `message.text_bodies()`, `html_bodies()` (parts) | `message.text_body()`, `html_body()`; `text_bodies()` and `html_bodies()` now yield text |
| `message.raw_message`, `raw_message()` | `message.raw()` (the input) or `message.root().raw()` (the range of the root message) |
| `message.header(name)` | `message.headers().value(name)` |
| `message.header_raw(name)` | `raw_value()` (bytes) of `message.headers().get(name)` |
| `message.header_values(name)` | `value()` of each field of `message.headers().all(name)` |
| `message.headers()` (a slice), `headers_raw()` | `message.headers()` (a `Headers` view, iterated with `for header in message.headers()` or `iter()`); `raw_name()` and `raw_value()` of each field |
| `message.remove_header(name)` | No replacement: messages are read-only |
| `message.is_empty()` | `parse()` returns `None` for such input; `headers().has_known()` |
| `from()`, `to()`, `cc()` and the other address getters (`Option<&Address>`) | `Option<AddressList>` |
| `date()` (`Option<&DateTime>`), `resent_date()` (`&HeaderValue`) | `Option<DateTime>` |
| `in_reply_to()`, `references()`, `keywords()`, `resent_message_id()`, `return_path()` (`&HeaderValue`) | `Option<TextList>` |
| `list_archive()` to `list_unsubscribe()` (`&HeaderValue`) | `Option<AddressList>` |
| `comments()`, `mime_version()` (`&HeaderValue`) | `Option<&str>` |
| `received()` (`Option<&Received>`) | `Option<Received>` |
| `received_all()` | `all_received()`, like `all_to()`, `all_cc()` and `all_bcc()` |
| `part.body` (`PartType`) | `part.kind()` (`PartKind`), with `text()`, `decoded()`, `nested()` and `children()` for the content |
| `part.contents()` | `part.decoded()` |
| `part.text_contents()` | `part.text()` |
| `part.message()` (the nested message) | `part.nested()`; `part.message()` now returns the message the part belongs to |
| `part.sub_parts()` | `part.children()` |
| `part.len()` | `part.decoded_len()` |
| `part.is_text_html()`, `part.is_binary()`, `part.is_empty()` | a match on `part.kind()`; `part.decoded_len() == 0` |
| `part.is_encoding_problem` | `part.has_problems()` and `part.flags()`; `decoded_checked()` and `text_checked()` |
| `part.encoding`, `part.headers` | `part.encoding()`, `part.headers()` |
| `part.offset_header`, `offset_body`, `offset_end`, `raw_header_offset()`, `raw_body_offset()`, `raw_end_offset()` | `part.offset_header()`, `offset_body()`, `offset_end()` |
| `part.raw_len()` | `part.raw().len()` |
| `MimeHeaders` methods (`content_type()`, `content_id()`, `attachment_name()`, ...) | Inherent methods of `MessagePart` and `MessageRef`; `content_language()` returns `Option<TextList>` |
| `PartType::Message(message)` | `PartKind::Message(message)`, a `MessageRef` |
| `header.name` (a `HeaderName`), `header.name()` (a `&str`) | `header.name()` (a `HeaderName`), `header.raw_name()` (as written) |
| `header.value`, `header.offset_*` | `header.value()`, `header.offset_field()`, `offset_start()`, `offset_end()` (same meaning) |
| `GetHeader::header()`, `header_value()` | `Headers::get()`, `Headers::value()` |
| `HeaderValue::Text(Cow<str>)` | `HeaderValue::Text(&str)` |
| `HeaderValue::TextList(Vec<Cow<str>>)` | `HeaderValue::TextList(TextList)`, also for a single item |
| `HeaderValue::Address(Address::List(..))`, `HeaderValue::Address(Address::Group(..))` | `HeaderValue::Address(AddressList)` with `iter()`, `mailboxes()` and `groups()` |
| `HeaderValue::Received(Box<Received>)` | `HeaderValue::Received(Received)`; hosts are returned by value |
| `value.unwrap_text()`, `into_text()` and the other `unwrap_*()` and `into_*()` | `as_text()`, `as_text_list()`, `as_address()`, `as_datetime()`, `as_content_type()`, `as_received()` |
| `Address::first()`, `last()`, `contains()`, `iter()` | `AddressList::first()`, `last()`, `contains()`, `mailboxes()` |
| `Address::as_list()`, `as_group()`, `into_list()`, `into_group()` | `AddressList::mailboxes()`, `AddressList::groups()` |
| `Addr` (`name`, `address`) | `Mailbox` (`name()`, `address()`) |
| `Group` (`name`, `addresses`) | `Group` (`name()`, `mailboxes()`) |
| `ContentType` (`c_type`, `c_subtype`, `attributes`) | `ContentType` (`ctype()`, `subtype()`, `attribute()`, `attributes()` yielding name and value) |
| `HeaderName::Other("X-Mailer")` and the other names added in 1.0 | `HeaderName::XMailer`; an `Other` with the same spelling still finds and configures the field, but `header.name()` returns the variant |
| `HeaderValue::Empty` for a blank Subject or other text field (`subject() == None`) | `HeaderValue::Text("")` (`subject() == Some("")`) |
| `HeaderName::from("X.Spam")` gave `Other("")` | `Other("X.Spam")`, which finds the field |
| `HeaderName::into_string()`, `len()` | `String::from(name)`, `name.as_str().len()` |
| `MessagePartId` | `PartId` |
| `decoders::base64::base64_decode()`, `base64_decode_slice()`, `base64_decode_stream()`, `BASE64_MAP` | `encodify::base64` |
| `decoders::quoted_printable::quoted_printable_decode()`, `quoted_printable_decode_char()`, `HEX_MAP` | `encodify::qp` |
| `decoders::hex::decode_hex()` | `encodify::hex` |
| `decoders::charsets::map::charset_decoder()` | `Charset::from_label()` and `Charset::decode()`; `decoders::charsets::charset_decoder()` is kept |
| `parsers::preview::preview_text()` and the other preview functions (taking `Cow<str>`) | `preview_text()` and the others at the crate root, taking `&str` |
| `parsers::fields::thread::thread_name()` | `thread_name()` |
| `parsers::fields::address::parse_address_local_part()` and the other address helpers | `parse_address_local_part()` and the others at the crate root |
| `rkyv` feature | No replacement |
| `base64_slice` feature | encodify |

## [0.11.9] - 2026-09-09

### Added
- Added `base64_decode_slice` behind the new `base64_slice` feature, which adds a `memchr` dependency.

## [0.11.8] - 2026-08-22

### Fixed
- `HeaderName` breaks rkyv serialization from <= 0.11.6.

## [0.11.7] - 2026-08-20

### Added
- Added more IANA headers.

### Fixed
- `DateTime::to_timezone` corrupting non-whole-hour offsets by storing leftover seconds in `tz_minute` (#158)

## [0.11.6] - 2026-08-10

### Fixed
- Missing whitespace between a quoted name and a following encoded word (#150)
- `panic` when a `Received` header ends with a folded line (#155)
- `Received` header tokens losing their last character at the end of the input, retaining folding characters, and failing to parse a clause folded before its value (#155)
- Multi-word display names followed by a comment no longer produce a fabricated address (#153)

## [0.11.5] - 2026-07-08

### Added
- Recognize additional charset labels supported by `encoding_rs` (#123)
- Added `Dkim2Signature` and `MessageInstance` to support DKIM2 headers.

### Fixed
- Address names containing LF are not parsed correctly (#149)
- Decode the `iso-8859-1` label as `windows-1252` per the WHATWG Encoding Standard (#131)
- `panic` with messages containing corrupted eml attachments (#120).

## [0.11.4] - 2026-06-21

### Added
- Add `Message::received_all()` to iterate over all Received header fields (#146)

### Changed
- Reject dates with invalid month names.
- Leniently decode quoted-printable bodies with invalid `=` escapes (#144)

### Fixed
- `parse_date()` doesn't handle UTC+12 and greater (#148)

## [0.11.3] - 2026-05-02

### Fixed
- Fix panic with messages containing corrupted attachments (#145)

## [0.11.2] - 2026-02-14

### Fixed
- Do not return invalid mime parts when parsing broken nested messages.
- Fix broken receive header date parsing when tab is used in long header syntax (#130)

## [0.11.1] - 2025-08-18

### Fixed
- Fix `DateTime::from_timestamp` to handle negative timestamps correctly.

## [0.11.0] - 2025-05-11

### Added
- `rkyv` zero-copy deserialization support.

### Changed
- Changed `usize` to `u32` types.
- Renamed `serde_support` feature to `serde`.

### Fixed
- Parsing of headers without LFs (#102)

## [0.10.2] - 2025-01-28

### Fixed
- Fixed `HeaderName` enum order to avoid breaking bincode serialization.

## [0.10.1] - 2025-01-26

### Fixed
- Fixed `HeaderName::parse` function.

## [0.10.0] - 2025-01-26

### Added
- Added `DkimSignature`, `ArcAuthenticationResults`, `ArcMessageSignature` and `ArcSeal` headers.

### Changed
- Perfect hashing using `hashify` crate rather than static `gperf` generated code.
- `HeaderName` is non-exhaustive.
- Parse obsolete timezones (#95).
- Retain mbox IO errors (#91).
- Hide concrete type behind impl type (#94).

### Removed
- Removed `ludicrous` feature, the Rust compiler is smart enough to optimize array lookups.

### Fixed
- Folding ws between "Content-Type:" and "plain/text" leads to empty header (#96).
- Multiline quoted continuations (closes #92).
- Deserialize (#93).

## [0.9.4] - 2024-09-04

### Changed
- Flexible parsing of charset names (#85).

## [0.9.3] - 2024-03-28

### Fixed
- Fixed parsing of address names containing @ (#80)

## [0.9.2] - 2023-11-27

### Fixed
- Fixed `quoted_printable_decode` external function (not used by mail-parser directly).
- Fix `Received` header serialization for bincode compatibility.

## [0.9.1] - 2023-09-22

### Changed
- Updated Rust edition to 2021.

### Removed
- Removed `content_type()` and `address()` functions that could `panic!`. Use `as_content_type()` and `as_address()` instead.

### Fixed
- Fixed panic when Content-Disposition is empty (#63)

## [0.9.0] - 2023-09-05

This version introduces multiple breaking changes. Please read the following notes carefully.

### Added
- Added parser for `Received` headers.
- Added `MessageParser::parse_headers` function to parse only the headers of a message.

### Changed
- Parsing is now done using `MessageParser`, which allows to customize the parsing process.
- All address types are now stored in the `HeaderValue::Address` variant using the `Address` enum.
- Renamed the `as_` prefix to `to_` in some functions.

### Removed
- Removed `RfcHeader` enum, now all headers are represented using `HeaderName`.

## [0.8.2] - 2023-02-22

### Fixed
- Parsing address name with \ characters (#41)
- Missing space when folded header begins with RFC2047 word (#43)

## [0.8.1] - 2023-02-09

### Added
- Added `raw_message()` function.

## [0.8.0] - 2022-12-01

### Changed
- Removed get_() prefixes (#31).
- Maildir import: Use modified time instead of created time (#32)

## [0.7.0] - 2022-10-21

### Added
- Automatic parsing of base64/qp encoded nested messages.
- Added "ludicrous mode" Cargo option to use some unsafe code for additional performance.

### Changed
- Base64/QuotedPrintable decoding optimizations.
- Refactoring or ``MessageStream`` to use iterators more efficiently.

### Fixed
- Fixed support for empty messages.
- Fixed raw offsets of multipart/* parts to include MIME epilogue.
- Fixed values of non-RFC headers.

## [0.6.1] - 2022-09-09

### Added
- Support for malformed unstructured fields containing encoded words (#29).
- Add support for gb2312 charsets (#30).

## [0.6.0] - 2022-08-21

### Added
- Maildir parsing support.
- Support for Content-Type attributes spanning multiple lines.
- Support for malformed Thunderbird messages (#27).

### Changed
- Headers and attributes are now stored in a `Vec` instead of a `HashMap` for a tiny performance enhancement.

### Fixed
- Fixed raw offset range for body parts.

## [0.5.0] - 2022-07-15

### Added
- Added raw offsets to MIME parts.

### Changed
- `Message` headers are now stored as a `MessagePart` with index 0.
- Improved `MessagePart` API.
- Nested base64/quoted-printable encoded message/rfc822 parts are automatically parsed when calling `get_message`.
- Better handling of malformed MIME messages.

## [0.4.8] - 2022-06-06

### Fixed
- get_bytes_to_boundary fix (#21)

## [0.4.7] - 2022-06-03

### Added
- Retrieving message headers in order (#19)
- Added `get_raw_headers` and `get_header` methods.
- Added `get_return_address` method to obtain the return address from the Return-Path or From headers.
- Support for malformed Return-Path headers.
- Support for ks_c_5601 charsets (#20)

## [0.4.6]

### Fixed
- DateTime is_valid() fix (#15)

## [0.4.5] - 2022-03-14

### Added
- DateTime to UNIX timestamp conversion.
- Ord, PartialOrd support for DateTime (#13).

### Fixed
- Fixed Message::parse() panic on duplicate Content-Type headers (#14).

## [0.4.4] - 2022-01-24

### Added
- Support for multi-line headers.
- Text and HTML message body preview.

### Changed
- Improved support for raw headers.

## [0.4.3] - 2021-12-31

### Added
- Mbox file parsing support (issue #11) conforming to the [QMail specification](http://qmail.org/qmail-manual-html/man5/mbox.html).
- Support for bincode serialize/deserialize.

## [0.4.2] - 2021-12-22

### Added
- Added `Message::get_thread_name()` to obtain the base subject of a message as defined in [RFC 5957 - Internet Message Access Protocol - SORT and THREAD Extensions (Section 2.1)](https://datatracker.ietf.org/doc/html/rfc5256#section-2.1).
- Added `MimeHeader::get_attachment_name` for simplified access to a MIME attachment file name.

## [0.4.1] - 2021-12-16

### Added
- Support for base64/quoted-printable nested messages.

### Changed
- Lazy parsing of nested e-mail messages.

## [0.4.0] - 2021-12-16

### Changed
- Lazy conversion to/from HTML an plain text parts.
- Improved API.
- Parts are now generics.

## [0.3.1] - 2021-12-15

### Added
- Support for non-standard headers.

### Changed
- Raw message offsets are stored in the message object.
- Message body structure is now stored in the message object.

## [0.3.0] - 2021-11-08

### Added
- Added support for new RFCs:
  - [RFC 2557 - MIME Encapsulation of Aggregate Documents, such as HTML (MHTML)](https://datatracker.ietf.org/doc/html/rfc2557)
  - [RFC 2392 - Content-ID and Message-ID Uniform Resource Locators](https://datatracker.ietf.org/doc/html/rfc2392)
  - [RFC 3282 - Content Language Headers](https://datatracker.ietf.org/doc/html/rfc3282)
  - [RFC 3339 - Date and Time on the Internet: Timestamps](https://datatracker.ietf.org/doc/html/rfc3339)

### Changed
- Improved API, now `Message::parse` returns `Option<Message>` to indicate when parsing was successful.
- Headers are now stored internally in a `HashMap` instead of `struct` fields.

## [0.2.2] - 2021-11-04

### Changed
- Improved decoder results.
- Dependency requirements for `encoding_rs`, `serde` and `serde_bytes` use caret ranges (`0.8`, `1.0` and `0.11`) instead of `>=0.8.28`, `>=1.0.117` and `>=0.11.5`.

## [0.2.1] - 2021-11-03

### Changed
- Performance enhacements, now *mail-parser* is almost as fast as the `unsafe` 0.1 version.

## [0.2.0] - 2021-11-03

### Added
- Added `Message::is_empty`.

### Changed
- Re-factoring to use **100% safe** Rust after a [discussion on Reddit](https://www.reddit.com/r/rust/comments/qkc5rk/fast_and_robust_email_parsing_library_for_rust/).

## [0.1.1] - 2021-11-02

### Fixed
- Bug-fixing after **fuzzing** the library.

## [0.1.0] - 2021-11-01

### Added
- Initial release with plenty of `unsafe` code to speed things up.
