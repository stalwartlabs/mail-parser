mail-parser 0.9.4
================================
- Flexible parsing of charset names (#85).

mail-parser 0.9.3
================================
- Fixed parsing of address names containing @ (#80)

mail-parser 0.9.2
================================
- Fixed `quoted_printable_decode` external function (not used by mail-parser directly).
- Fix `Received` header serialization for bincode compatibility.

mail-parser 0.9.1
================================
- Fixed panic when Content-Disposition is empty (#63)
- Removed `content_type()` and `address()` functions that could `panic!`. Use `as_content_type()` and `as_address()` instead.
- Updated Rust edition to 2021.

mail-parser 0.9.0
================================
This version introduces multiple breaking changes. Please read the following notes carefully.

- Parsing is now done using `MessageParser`, which allows to customize the parsing process.
- Added parser for `Received` headers.
- Added `MessageParser::parse_headers` function to parse only the headers of a message.
- Removed `RfcHeader` enum, now all headers are represented using `HeaderName`.
- All address types are now stored in the `HeaderValue::Address` variant using the `Address` enum.
- Renamed the `as_` prefix to `to_` in some functions.

mail-parser 0.8.2
================================
- Fix: Parsing address name with \ characters (#41) 
- Fix: Missing space when folded header begins with RFC2047 word (#43) 

mail-parser 0.8.1
================================
- Added `raw_message()` function.

mail-parser 0.8.0
================================
- Removed get_() prefixes (#31).
- Maildir import: Use modified time instead of created time (#32)

mail-parser 0.7.0
================================
- Base64/QuotedPrintable decoding optimizations.
- Automatic parsing of base64/qp encoded nested messages.
- Refactoring or ``MessageStream`` to use iterators more efficiently.
- Added "ludicrous mode" Cargo option to use some unsafe code for additional performance.
- Fixed support for empty messages.
- Fixed raw offsets of multipart/* parts to include MIME epilogue.
- Fixed values of non-RFC headers.

mail-parser 0.6.1
================================
- Support for malformed unstructured fields containing encoded words (#29).
- Add support for gb2312 charsets (#30).

mail-parser 0.6.0
================================
- Maildir parsing support.
- Headers and attributes are now stored in a `Vec` instead of a `HashMap` for a tiny performance enhancement.
- Support for Content-Type attributes spanning multiple lines.
- Support for malformed Thunderbird messages (#27). 
- Fixed raw offset range for body parts.

mail-parser 0.5.0
================================
- `Message` headers are now stored as a `MessagePart` with index 0.
- Improved `MessagePart` API.
- Nested base64/quoted-printable encoded message/rfc822 parts are automatically parsed when calling `get_message`.
- Better handling of malformed MIME messages.
- Added raw offsets to MIME parts.

mail-parser 0.4.8
================================
- get_bytes_to_boundary fix (#21)

mail-parser 0.4.7
================================
- Retrieving message headers in order (#19)
- Added `get_raw_headers` and `get_header` methods.
- Added `get_return_address` method to obtain the return address from the Return-Path or From headers.
- Support for malformed Return-Path headers.
- Support for ks_c_5601 charsets (#20)

mail-parser 0.4.6
================================
- DateTime is_valid() fix (#15)
  
mail-parser 0.4.5
================================
- DateTime to UNIX timestamp conversion.
- Ord, PartialOrd support for DateTime (#13).
- Fixed Message::parse() panic on duplicate Content-Type headers (#14).

mail-parser 0.4.4
================================
- Support for multi-line headers.
- Text and HTML message body preview.
- Improved support for raw headers.

mail-parser 0.4.3
================================
- Mbox file parsing support (issue #11) conforming to the [QMail specification](http://qmail.org/qmail-manual-html/man5/mbox.html).
- Support for bincode serialize/deserialize.

mail-parser 0.4.2
================================
- Added `Message::get_thread_name()` to obtain the base subject of a message as defined in [RFC 5957 - Internet Message Access Protocol - SORT and THREAD Extensions (Section 2.1)](https://datatracker.ietf.org/doc/html/rfc5256#section-2.1).
- Added `MimeHeader::get_attachment_name` for simplified access to a MIME attachment file name.

mail-parser 0.4.1
================================
- Lazy parsing of nested e-mail messages.
- Support for base64/quoted-printable nested messages.

mail-parser 0.4.0
================================
- Lazy conversion to/from HTML an plain text parts.
- Improved API.
- Parts are now generics.

mail-parser 0.3.1
================================
- Support for non-standard headers.
- Raw message offsets are stored in the message object.
- Message body structure is now stored in the message object.

mail-parser 0.3
================================
- Improved API, now `Message::parse` returns `Option<Message>` to indicate when parsing was successful.
- Headers are now stored internally in a `HashMap` instead of `struct` fields.
- Added support for new RFCs:
  - [RFC 2557 - MIME Encapsulation of Aggregate Documents, such as HTML (MHTML)](https://datatracker.ietf.org/doc/html/rfc2557)
  - [RFC 2392 - Content-ID and Message-ID Uniform Resource Locators](https://datatracker.ietf.org/doc/html/rfc2392)
  - [RFC 3282 - Content Language Headers](https://datatracker.ietf.org/doc/html/rfc3282)
  - [RFC 3339 - Date and Time on the Internet: Timestamps](https://datatracker.ietf.org/doc/html/rfc3339)

mail-parser 0.2.1
================================
- Performance enhacements, now *mail-parser* is almost as fast as the `unsafe` 0.1 version.

mail-parser 0.2
================================
- Re-factoring to use **100% safe** Rust after a [discussion on Reddit](https://www.reddit.com/r/rust/comments/qkc5rk/fast_and_robust_email_parsing_library_for_rust/).
- Added `Message::is_empty`.

mail-parser 0.1.1
================================
- Bug-fixing after **fuzzing** the library.

mail-parser 0.1
================================
- Initial release with plenty of `unsafe` code to speed things up.






