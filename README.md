# mail-parser

[![crates.io](https://img.shields.io/crates/v/mail-parser)](https://crates.io/crates/mail-parser)
[![build](https://github.com/stalwartlabs/mail-parser/actions/workflows/rust.yml/badge.svg)](https://github.com/stalwartlabs/mail-parser/actions/workflows/rust.yml)
[![docs.rs](https://img.shields.io/docsrs/mail-parser)](https://docs.rs/mail-parser)
[![msrv](https://img.shields.io/crates/msrv/mail-parser)](https://crates.io/crates/mail-parser)
[![crates.io](https://img.shields.io/crates/l/mail-parser)](http://www.apache.org/licenses/LICENSE-2.0)

_mail-parser_ is an **e-mail parsing library** written in Rust that fully conforms to the Internet Message Format standard (_RFC 5322_), the Multipurpose Internet Mail Extensions (MIME; _RFC 2045 - 2049_) as well as many other [internet messaging RFCs](#conformed-rfcs).

It also supports decoding messages in [41 different character sets](#supported-character-sets) including obsolete formats such as UTF-7. All Unicode (UTF-*) and single-byte character sets are handled internally by the library while support for legacy multi-byte encodings of Chinese and Japanese languages such as BIG5 or ISO-2022-JP is provided by the optional dependency [encoding_rs](https://crates.io/crates/encoding_rs).

In general, this library abides by the Postel's law or [Robustness Principle](https://en.wikipedia.org/wiki/Robustness_principle) which states that an implementation must be conservative in its sending behavior and liberal in its receiving behavior. This means that _mail-parser_ will make a best effort to parse non-conformant e-mail messages as long as these do not deviate too much from the standard.

Besides the MIME tree, every message comes with the three body lists of [RFC 8621, Section 4.1.4](https://datatracker.ietf.org/doc/html/rfc8621#section-4.1.4): the text body, the HTML body and the attachments. HTML is converted to plain text, and plain text to HTML, when a part has no alternative in the other format.

## Features

- **Fast parsing**:
    - Boundaries and header fields are found by SIMD kernels (NEON on aarch64, AVX2 and SSE2 on x86-64, `memchr` on every other target). 
    - Base64 and quoted-printable are decoded by the SIMD codecs of [encodify](https://github.com/stalwartlabs/encodify)
    - UTF-8 is validated by [simdutf8](https://crates.io/crates/simdutf8)
    - Header names, charset labels and HTML entities are looked up with perfect hashing through [hashify](https://crates.io/crates/hashify).
- **Lazy decoding**: Nothing is transfer-decoded or converted from its character set during the parse; a message whose attachments are never read never pays for decoding them.
- **Zero-copy**: Practically all strings returned by this library are references to the input raw message.
- **Battle-tested** with millions of real-world e-mail messages dating from 1995 until today. 
- Every function in the library has been [fuzzed](#testing-fuzzing--benchmarking) and thoroughly [tested with MIRI](#testing-fuzzing--benchmarking).
- Used in production environments worldwide by [Stalwart Mail Server](https://github.com/stalwartlabs/mail-server).

> Note: This library does not support building e-mail messages as this functionality is provided separately by the [`mail-builder`](https://crates.io/crates/mail-builder) crate.

## Usage

### Parse a message

```rust
use mail_parser::{Address, MessageParser, PartKind};

let input = br#"From: Art Vandelay <art@vandelay.com> (Vandelay Industries)
To: "Colleagues": "James Smythe" <james@vandelay.com>; Friends:
    jane@example.com, =?UTF-8?Q?John_Sm=C3=AEth?= <john@example.com>;
Date: Sat, 20 Nov 2021 14:22:01 -0800
Subject: Why not both importing AND exporting? =?utf-8?b?4pi6?=
Content-Type: multipart/mixed; boundary="festivus";

--festivus
Content-Type: text/html; charset="us-ascii"
Content-Transfer-Encoding: base64

PGh0bWw+PHA+SSB3YXMgdGhpbmtpbmcgYWJvdXQgcXVpdHRpbmcgdGhlICZsZHF1bztle
HBvcnRpbmcmcmRxdW87IHRvIGZvY3VzIGp1c3Qgb24gdGhlICZsZHF1bztpbXBvcnRpbm
cmcmRxdW87LDwvcD48cD5idXQgdGhlbiBJIHRob3VnaHQsIHdoeSBub3QgZG8gYm90aD8
gJiN4MjYzQTs8L3A+PC9odG1sPg==
--festivus
Content-Type: message/rfc822

From: "Cosmo Kramer" <kramer@kramerica.com>
Subject: Exporting my book about coffee tables
Content-Type: multipart/mixed; boundary="giddyup";

--giddyup
Content-Type: text/plain; charset="utf-16"
Content-Transfer-Encoding: quoted-printable

=FF=FE=0C!5=D8"=DD5=D8)=DD5=D8-=DD =005=D8*=DD5=D8"=DD =005=D8"=
=DD5=D85=DD5=D8-=DD5=D8,=DD5=D8/=DD5=D81=DD =005=D8*=DD5=D86=DD =
=005=D8=1F=DD5=D8,=DD5=D8,=DD5=D8(=DD =005=D8-=DD5=D8)=DD5=D8"=
=DD5=D8=1E=DD5=D80=DD5=D8"=DD!=00
--giddyup
Content-Type: image/gif; name*1="about "; name*0="Book ";
              name*2*=utf-8''%e2%98%95 tables.gif
Content-Transfer-Encoding: Base64
Content-Disposition: attachment

R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7
--giddyup--
--festivus--
"#;

let message = MessageParser::default().parse(input).expect("a message");

let sender = message.from().and_then(|from| from.first()).expect("a sender");
assert_eq!(sender.name(), Some("Art Vandelay (Vandelay Industries)"));
assert_eq!(sender.address(), Some("art@vandelay.com"));

let to = message.to().expect("recipients");
let addresses: Vec<_> = to.mailboxes().filter_map(|mailbox| mailbox.address()).collect();
assert_eq!(addresses, ["james@vandelay.com", "jane@example.com", "john@example.com"]);
let Some(Address::Group(friends)) = to.iter().nth(1) else {
    panic!("the second item is a group");
};
assert_eq!(friends.name(), Some("Friends"));
assert_eq!(friends.mailboxes().nth(1).and_then(|mailbox| mailbox.name()), Some("John Smîth"));

assert_eq!(message.date().expect("a date").to_rfc3339(), "2021-11-20T14:22:01-08:00");
assert_eq!(message.subject(), Some("Why not both importing AND exporting? ☺"));

assert_eq!(
    message.body_html(0).as_deref(),
    Some(concat!(
        "<html><p>I was thinking about quitting the &ldquo;exporting&rdquo; to ",
        "focus just on the &ldquo;importing&rdquo;,</p><p>but then I thought,",
        " why not do both? &#x263A;</p></html>"
    ))
);
assert_eq!(
    message.body_text(0).as_deref(),
    Some(concat!(
        "I was thinking about quitting the “exporting” to focus just on the",
        " “importing”,\nbut then I thought, why not do both? ☺\n"
    ))
);

let nested = message
    .attachments()
    .find_map(|part| part.nested())
    .expect("a nested message");
assert_eq!(nested.subject(), Some("Exporting my book about coffee tables"));
assert_eq!(nested.body_text(0).as_deref(), Some("ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!"));
assert_eq!(
    nested.body_html(0).as_deref(),
    Some("<html><body>ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!</body></html>")
);

let image = nested.attachments().next().expect("an attachment");
assert_eq!(image.attachment_name(), Some("Book about ☕ tables.gif"));
assert!(matches!(image.kind(), PartKind::Binary));
assert_eq!(image.decoded_len(), 42);
```

### Read header fields

```rust
use mail_parser::{HeaderName, HeaderValue, MessageParser};

let raw = b"Received: from mx.example.com by mail.example.org; Sat, 20 Nov 2021 14:22:05 -0800\r\n\
Message-ID: <1234@example.com>\r\n\
X-Priority: 1\r\n\
X-Custom: first\r\n\
X-Custom: second\r\n\
Subject: Status report\r\n\
\r\n\
Body\r\n";
let message = MessageParser::new().parse(raw).expect("a message");
let headers = message.headers();

assert_eq!(message.message_id(), Some("1234@example.com"));
assert_eq!(headers.get("x-custom").map(|header| header.raw_value()), Some(&b" second\r\n"[..]));
assert_eq!(headers.all("X-Custom").count(), 2);
assert!(headers.contains(HeaderName::XPriority));

for header in headers {
    match header.value() {
        HeaderValue::Text(text) => println!("{}: {text}", header.raw_name()),
        HeaderValue::Received(received) => println!("received by {:?}", received.by()),
        other => println!("{}: {other:?}", header.raw_name()),
    }
}
```

### Decode bodies on demand

```rust
use mail_parser::{DecodeProblems, MessageParser};

let raw = concat!(
    "Content-Type: multipart/mixed; boundary=\"b\"\r\n",
    "\r\n",
    "--b\r\n",
    "Content-Type: text/plain; charset=iso-8859-1\r\n",
    "Content-Transfer-Encoding: quoted-printable\r\n",
    "\r\n",
    "Caf=E9 cr=E8me\r\n",
    "--b\r\n",
    "Content-Type: application/octet-stream\r\n",
    "Content-Transfer-Encoding: base64\r\n",
    "\r\n",
    "AAEC*AwQF\r\n",
    "--b--\r\n",
);
let message = MessageParser::new().parse(raw).expect("a message");

let text = message.text_body().next().expect("a text part");
assert_eq!(text.text().as_deref(), Some("Café crème"));
assert_eq!(text.text_prefix(4).as_deref(), Some("Café"));

let file = message.attachments().next().expect("an attachment");
assert_eq!(file.decoded().as_ref(), b"\x00\x01\x02\x03\x04\x05");
assert_eq!(file.decoded_len(), 6);
let (_, problems) = file.decoded_checked();
assert!(problems.contains(DecodeProblems::MALFORMED_TRANSFER_ENCODING));
```

### Walk the MIME structure

```rust
use mail_parser::{MessageParser, Source};

let raw = concat!(
    "Content-Type: multipart/mixed; boundary=\"outer\"\r\n",
    "\r\n",
    "--outer\r\n",
    "Content-Type: multipart/alternative; boundary=\"inner\"\r\n",
    "\r\n",
    "--inner\r\n",
    "Content-Type: text/plain\r\n",
    "\r\n",
    "Hello\r\n",
    "--inner\r\n",
    "Content-Type: text/html\r\n",
    "\r\n",
    "<p>Hello</p>\r\n",
    "--inner--\r\n",
    "--outer\r\n",
    "Content-Type: message/rfc822\r\n",
    "Content-Transfer-Encoding: base64\r\n",
    "\r\n",
    "U3ViamVjdDogRm9yd2FyZGVkDQoNCkJvZHkNCg==\r\n",
    "--outer--\r\n",
);
let message = MessageParser::new().parse(raw).expect("a message");

let alternative = message.part_by_boundary("inner").expect("a multipart");
assert_eq!(alternative.id(), 1);
assert_eq!(alternative.delimiter(), Some("outer"));
let html = alternative.children().nth(1).expect("the HTML part");
assert_eq!(html.delimiter(), Some("inner"));
assert_eq!(html.raw_body(), b"<p>Hello</p>");

let forwarded = message.parts().find_map(|part| part.nested()).expect("a nested message");
assert_eq!(forwarded.subject(), Some("Forwarded"));
assert_eq!(forwarded.container().map(|part| part.id()), Some(4));
assert_eq!(forwarded.source(), Source::Decoded(4));
assert_eq!(forwarded.body_text(0).as_deref(), Some("Body\r\n"));
assert_eq!(message.parts().len(), 6);
```

### Parse many messages

```rust
use mail_parser::{MessageBuffers, MessageParser, mailbox::mbox::MessageIterator};

let mbox = b"From ann@example.com Sat Jan  3 01:05:34 1996\n\
Subject: first\n\
\n\
Hello\n\
From bob@example.com Sat Jan  3 01:06:10 1996\n\
Subject: second\n\
\n\
Hi\n";

let parser = MessageParser::new();
let mut buffers = MessageBuffers::new();
let mut subjects = Vec::new();
for entry in MessageIterator::new(&mbox[..]) {
    let entry = entry.expect("a readable mbox");
    if let Some(message) = parser.parse_with(entry.contents(), &mut buffers) {
        subjects.push(message.subject().unwrap_or_default().to_string());
        buffers = message.into_buffers();
    }
}
assert_eq!(subjects, ["first", "second"]);
```

### Parse header values on their own

```rust
use mail_parser::{DateTime, HeaderForm};

let parsed = HeaderForm::Addresses.parse(b" Ann <ann@example.com>, bob@example.com\r\n");
let list = parsed.value().as_address().expect("an address list");
assert_eq!(list.mailboxes().count(), 2);

let subject = HeaderForm::Text.parse(b" =?utf-8?q?caf=C3=A9?= au lait\r\n");
assert_eq!(subject.value().as_text(), Some("café au lait"));

let date = DateTime::parse_rfc822("Sat, 20 Nov 2021 14:22:01 -0800").expect("a date");
assert_eq!(date.to_timestamp(), 1_637_446_921);
```

### Configure the parser

```rust
use mail_parser::{HeaderForm, HeaderName, MessageParser};

let parser = MessageParser::new()
    .header(HeaderName::Received, HeaderForm::Raw)
    .header("X-Sender", HeaderForm::Addresses)
    .unknown_headers(HeaderForm::Ignore)
    .max_depth(16)
    .max_parts(500);
let message = parser
    .parse(b"X-Sender: <kel@vandelay.com>\r\nX-Spam: yes\r\n\r\nbody")
    .expect("a message");
let sender = message
    .headers()
    .value("X-Sender")
    .and_then(|value| value.as_address())
    .and_then(|list| list.first());
assert_eq!(sender.and_then(|mailbox| mailbox.address()), Some("kel@vandelay.com"));
assert!(message.headers().value("X-Spam").is_some_and(|value| value.is_empty()));
```

Build a parser once and share it; it is `Send` and `Sync`. `header(name, form)` sets the form of one header field, `unknown_headers(form)` the form of every field with a name the parser does not know. Content-Type, Content-Disposition and Content-Transfer-Encoding always keep their MIME form. The defaults are the forms 0.11 used:

| Form | Header fields |
|---|---|
| `Text` | Subject, Comments, Content-Description, Content-Location, Content-Transfer-Encoding |
| `Addresses` | From, To, Cc, Bcc, Reply-To, Sender, Resent-From, Resent-To, Resent-Cc, Resent-Bcc, Resent-Sender, List-Archive, List-Help, List-ID, List-Owner, List-Post, List-Subscribe, List-Unsubscribe |
| `MessageIds` | Message-ID, In-Reply-To, References, Return-Path, Content-ID, Resent-Message-ID |
| `CommaList` | Keywords, Content-Language |
| `Date` | Date, Resent-Date |
| `ContentType` | Content-Type, Content-Disposition |
| `Received` | Received |
| `Raw` | every other field |

`Ignore` keeps a field with an empty value, and any form can be set for any name. `parse_headers()` stops after the header block of the message, for callers that need no MIME structure, such as DKIM verification. The limits bound the nesting depth (64 by default), the number of parts (10,000) and the levels of transfer-encoded nested messages that are decoded and parsed (3); content past a limit stays in the last part, which is flagged.

### Keep a message

```rust
use mail_parser::{Message, MessageParser};

let raw = b"Subject: moved\r\n\r\nbody".to_vec();
let message: Message<'static> = MessageParser::new().parse_owned(raw).expect("a message");
let subject = std::thread::spawn(move || message.subject().map(str::to_string))
    .join()
    .expect("the thread finished");
assert_eq!(subject.as_deref(), Some("moved"));
```

`parse_owned()` takes ownership of the input, and `into_owned()` copies a borrowed input; either way only the raw bytes are copied or moved, never the parsed values.

## Performance

**[PLACEHOLDER: performance section, to be written after the final benchmarks against 0.11 and against other Rust and C/C++ parsers.]**

## Feature flags

| Feature | Effect |
|---|---|
| `full_encoding` | Decodes the multi-byte character sets through `encoding_rs`: Shift_JIS, EUC-JP, ISO-2022-JP, Big5, GBK, GB18030, EUC-KR, windows-874, IBM866, x-mac-cyrillic and x-user-defined. Without it, text in these character sets is decoded as UTF-8 and reported as an unknown charset by `text_checked()`. |
| `serde` | `Serialize` for `Message` and the views; `Serialize` and `Deserialize` for `DateTime`, `HeaderName` and the Received enums. |

No feature is enabled by default. The minimum supported Rust version is 1.98.

## Testing, Fuzzing & Benchmarking

To run the test suite, including the comparison of every message under `resources/eml/` with its expected JSON (in LF and CRLF form; a mismatch writes the parsed message to a `.failed` file next to it):

```bash
 $ cargo test --features full_encoding,serde
```

and without optional features:

```bash
 $ cargo test
```

**[PLACEHOLDER: fuzzing and Miri commands, to be written after the final fuzzing and Miri runs.]**

To run the micro-benchmarks of the SIMD kernels and of string resolution:

```bash
 $ cargo bench --bench kernels
 $ cargo bench --bench resolve
```

## Conformed RFCs

- [RFC 822 - Standard for ARPA Internet Text Messages](https://datatracker.ietf.org/doc/html/rfc822)
- [RFC 5322 - Internet Message Format](https://datatracker.ietf.org/doc/html/rfc5322)
- [RFC 2045 - Multipurpose Internet Mail Extensions (MIME) Part One: Format of Internet Message Bodies](https://datatracker.ietf.org/doc/html/rfc2045)
- [RFC 2046 - Multipurpose Internet Mail Extensions (MIME) Part Two: Media Types](https://datatracker.ietf.org/doc/html/rfc2046)
- [RFC 2047 - MIME (Multipurpose Internet Mail Extensions) Part Three: Message Header Extensions for Non-ASCII Text](https://datatracker.ietf.org/doc/html/rfc2047)
- [RFC 2048 - Multipurpose Internet Mail Extensions (MIME) Part Four: Registration Procedures](https://datatracker.ietf.org/doc/html/rfc2048)
- [RFC 2049 - Multipurpose Internet Mail Extensions (MIME) Part Five: Conformance Criteria and Examples](https://datatracker.ietf.org/doc/html/rfc2049)
- [RFC 2231 - MIME Parameter Value and Encoded Word Extensions: Character Sets, Languages, and Continuations](https://datatracker.ietf.org/doc/html/rfc2231)
- [RFC 2557 - MIME Encapsulation of Aggregate Documents, such as HTML (MHTML)](https://datatracker.ietf.org/doc/html/rfc2557)
- [RFC 2183 - Communicating Presentation Information in Internet Messages: The Content-Disposition Header Field](https://datatracker.ietf.org/doc/html/rfc2183)
- [RFC 2392 - Content-ID and Message-ID Uniform Resource Locators](https://datatracker.ietf.org/doc/html/rfc2392)
- [RFC 3282 - Content Language Headers](https://datatracker.ietf.org/doc/html/rfc3282)
- [RFC 6532 - Internationalized Email Headers](https://datatracker.ietf.org/doc/html/rfc6532)
- [RFC 2152 - UTF-7 - A Mail-Safe Transformation Format of Unicode](https://datatracker.ietf.org/doc/html/rfc2152)
- [RFC 2369 - The Use of URLs as Meta-Syntax for Core Mail List Commands and their Transport through Message Header Fields](https://datatracker.ietf.org/doc/html/rfc2369)
- [RFC 2919 - List-Id: A Structured Field and Namespace for the Identification of Mailing Lists](https://datatracker.ietf.org/doc/html/rfc2919)
- [RFC 5321 - Simple Mail Transfer Protocol (Section 4.4, trace information)](https://datatracker.ietf.org/doc/html/rfc5321#section-4.4)
- [RFC 3848 - ESMTP and LMTP Transmission Types Registration](https://datatracker.ietf.org/doc/html/rfc3848)
- [RFC 3339 - Date and Time on the Internet: Timestamps](https://datatracker.ietf.org/doc/html/rfc3339)
- [RFC 5233 - Sieve Email Filtering: Subaddress Extension](https://datatracker.ietf.org/doc/html/rfc5233)
- [RFC 8621 - The JSON Meta Application Protocol (JMAP) for Mail (Section 4.1.4)](https://datatracker.ietf.org/doc/html/rfc8621#section-4.1.4)
- [RFC 5256 - Internet Message Access Protocol - SORT and THREAD Extensions (Section 2.1)](https://datatracker.ietf.org/doc/html/rfc5256#section-2.1)

## Supported Character Sets

Every label of the [WHATWG Encoding Standard](https://encoding.spec.whatwg.org/#names-and-labels) is recognized, along with the MIME names of the character sets below. As in that standard, text labeled US-ASCII or ISO-8859-1 is decoded as windows-1252, TIS-620 covers ISO-8859-11, and with the `full_encoding` feature the labels mapped to the replacement encoding (ISO-2022-KR, ISO-2022-CN, HZ-GB-2312) turn the text into a single U+FFFD.

- UTF-8
- UTF-16, UTF-16BE, UTF-16LE
- UTF-7
- US-ASCII
- ISO-8859-1
- ISO-8859-2
- ISO-8859-3
- ISO-8859-4
- ISO-8859-5
- ISO-8859-6
- ISO-8859-7
- ISO-8859-8
- ISO-8859-9
- ISO-8859-10
- ISO-8859-13
- ISO-8859-14
- ISO-8859-15
- ISO-8859-16
- windows-1250
- windows-1251
- windows-1252
- windows-1253
- windows-1254
- windows-1255
- windows-1256
- windows-1257
- windows-1258
- KOI8-R
- KOI8-U
- Macintosh (Mac OS Roman)
- IBM850
- TIS-620

With the `full_encoding` feature, through [encoding_rs](https://crates.io/crates/encoding_rs):

- Shift_JIS
- EUC-JP
- ISO-2022-JP
- Big5
- GBK (GB2312)
- GB18030
- EUC-KR
- windows-874
- IBM866
- x-mac-cyrillic
- x-user-defined

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSES/Apache-2.0.txt](LICENSES/Apache-2.0.txt) or <https://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSES/MIT.txt](LICENSES/MIT.txt) or <https://opensource.org/licenses/MIT>)

at your option.

## Copyright

Copyright (C) 2020, Stalwart Labs LLC
