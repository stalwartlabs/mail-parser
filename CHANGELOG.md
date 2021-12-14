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






