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






