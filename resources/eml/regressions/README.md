# Regression fixtures

Small messages that used to be written inline in the Rust tests. Each
`NAME.eml` has the expected parse in `NAME.json` (LF line endings) and
`NAME.crlf.json` (CRLF line endings), compared by
`tests/integration_test.rs` (`parse_full_messages`, features `serde` and
`full_encoding`). Tests that also check API behaviour the JSON does not
show read the `.eml` file directly; they are named below.

Review findings refer to the 1.0 adversarial review (C, H, M, L and D
numbers); issue numbers refer to GitHub issues of `stalwartlabs/mail-parser`.

| File | Origin |
|------|--------|
| `readme-example` | The README example (formerly `MESSAGE` in `tests/integration_test.rs`); still read by `test_api` and `test_api_field_parsers`. |
| `alternative-attachment-disposition` | #67, RFC 8621 parseStructure: an alternative child with `Content-Disposition: attachment` is not a body. |
| `alternative-named-html` | #67: an alternative child with a `name` parameter is an attachment. |
| `mixed-image` | #67: inline media in `multipart/mixed` goes to both body lists and is not an attachment. |
| `alternative-related-image` | #67: image inside `multipart/related` inside `multipart/alternative`. |
| `inline-text-with-filename` | #67: a `filename` parameter counts as a name. |
| `alternative-dropped-footer` | #67 and #107: a text part in no list is dropped (`other_parts`). |
| `root-attachment-disposition` | #67: `Content-Disposition: attachment` applies to a single-part root. |
| `unterminated-alternative` | #67: an unterminated `multipart/alternative` still yields its bodies. |
| `alternative-nested-dropped` | #67: `multipart/alternative` respects the parts its children dropped. |
| `unopened-multiparts` | #67: multiparts with no boundary or no delimiter are attachments. |
| `related-cid-inline-image` | #70 and RFC 8621 `hasAttachment`: a cid image marked inline in `multipart/related`. |
| `related-cid-image-no-disposition` | RFC 8621 `hasAttachment`: a cid image without disposition in `multipart/related`. |
| `mixed-image-attachment` | RFC 8621 `hasAttachment`: image with disposition attachment in `multipart/mixed`. |
| `mixed-image-inline` | RFC 8621 `hasAttachment`: image with disposition inline in `multipart/mixed`. |
| `mixed-pdf` | RFC 8621 `hasAttachment`: PDF without disposition. |
| `mixed-pdf-inline` | RFC 8621 `hasAttachment`: PDF with disposition `INLINE`. |
| `mixed-calendar` | RFC 8621 `hasAttachment`: calendar part without disposition. |
| `mixed-message` | RFC 8621 `hasAttachment`: attached message without disposition. |
| `mixed-message-inline` | RFC 8621 `hasAttachment`: attached message with disposition inline. |
| `single-text` | RFC 8621 `hasAttachment`: single text part. |
| `related-disposition` | #70: the disposition decides `is_inline` (the `inline` role) in `multipart/related`. |
| `mixed-disposition` | #70: the disposition decides `is_inline` (the `inline` role) in `multipart/mixed`. |
| `mixed-text-image-html` | #73: text and HTML bodies converted for `text_bodies` and `html_bodies`; read by `issue_73_text_bodies_yield_text`. |
| `close-delimiter-first` | Review M6: the first delimiter is the close delimiter. |
| `close-delimiter-mid-line` | Review M6: a close delimiter mid-line before the line-start one. |
| `fallback-delimiter-after-text` | Review M6: a mid-line delimiter opens the multipart (`fallback_delimiter`). |
| `boundary-of-dashes` | Review M6: a boundary made of dashes never matches. |
| `no-delimiter-close-mid-line` | Review M6: only a mid-line close delimiter. |
| `nested-headers-cut-by-delimiter` | Review M7: a delimiter cuts the header block of a nested message. |
| `inner-headers-cut-by-outer-delimiter` | Review M7: an outer delimiter cuts the header block of an inner part. |
| `empty-part-then-cut-part` | Review M7: an empty part followed by a part cut by the close delimiter. |
| `empty-nested-message` | Review L2: an empty `message/rfc822` stays a text part. |
| `empty-nested-message-base64` | Review L2: an encoded `message/rfc822` with no header stays a binary part. |
| `doubly-encoded-message` | Review L4: sources of a message encoded inside an encoded message. |
| `garbage-line-before-fold` | Review H1: a colon-less first line followed by a folded field. |
| `empty-subject` | Review D1: an empty Subject is `Text("")`; read by `empty_text_and_raw_fields_are_empty_strings` (`thread_name`). |
| `empty-subject-blanks` | Review D1: a Subject of blanks; read as above. |
| `empty-subject-folded` | Review D1: a Subject of a folded blank line; read as above. |
| `empty-structured-fields` | Review D1: empty unstructured and structured fields. |
| `header-name-space` | Review D2: a space inside a field name; read by `header_names_are_strict`. |
| `header-name-leading-colon` | Review D2: a leading colon; read by `header_names_are_strict`. |
| `header-name-colons-and-blanks` | Review D2: leading colons and blanks; read by `header_names_are_strict`. |
| `header-name-form-feed` | Review D2: a control character inside a field name; read by `header_names_are_strict`. |
| `header-name-nul` | Review D2: a NUL inside a field name; read by `header_names_are_strict`. |
| `header-name-blanks-before-colon` | Review D2: blanks before the colon are still accepted (RFC 5322 obsolete syntax). |
| `header-name-smuggled-subject` | Review D2: `Sub ject` is not `Subject`; read by `header_names_are_strict` (`has_known`). |
| `utf7-terminators` | Review M2: UTF-7 keeps the character that ends a base64 run; one part per case. |
| `utf16-dangling-byte` | Review L1: a dangling UTF-16 byte becomes U+FFFD; one part per case; read by `utf16_dangling_byte_is_replaced`. |
| `date-zone-ut` | Date parser: the `UT` zone at the end of the field does not swallow the next line. |
| `date-zone-z` | Date parser: the same with `Z`. |
| `date-zone-military` | Date parser: the same with the military zone `A`. |
| `date-zone-est` | Date parser: the same with `EST`. |
| `date-without-final-newline` | Date parser: a Date field at the end of the input, no line break. |
| `unstructured-fields` | Unstructured parser: Subject, Comments and Content-Description with encoded words and folds. |
