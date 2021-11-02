# mail-parser

[![crates.io](https://img.shields.io/crates/v/mail-parser)](https://crates.io/crates/mail-parser)
[![build](https://github.com/stalwartlabs/mail-parser/actions/workflows/rust.yml/badge.svg)](https://github.com/stalwartlabs/mail-parser/actions/workflows/rust.yml)
[![docs.rs](https://img.shields.io/docsrs/mail-parser)](https://docs.rs/mail-parser)
[![crates.io](https://img.shields.io/crates/l/mail-parser)](http://www.apache.org/licenses/LICENSE-2.0)
[![Twitter Follow](https://img.shields.io/twitter/follow/stalwartlabs?style=social)](https://twitter.com/stalwartlabs)

_mail-parser_ is an **e-mail parsing library** written in Rust that fully conforms to the Internet Message Format standard (_RFC 5322_), the
Multipurpose Internet Mail Extensions (MIME; _RFC 2045 - 2049_) as well as other [internet messaging RFCs](#conformed-rfcs).

It also supports decoding messages in [41 different character sets](#supported-character-sets) including obsolete formats such as UTF-7.
All Unicode (UTF-*) and single-byte character sets are handled internally by the library while support for legacy multi-byte encodings of Chinese
and Japanese languages such as BIG5 or ISO-2022-JP is provided by the optional dependency [encoding_rs](https://crates.io/crates/encoding_rs).

In general, this library abides by the Postel's law or [Robustness Principle](https://en.wikipedia.org/wiki/Robustness_principle) which 
states that an implementation must be conservative in its sending behavior and liberal in its receiving behavior. This means that
_mail-parser_ will make a best effort to parse non-conformant e-mail messages as long as these do not deviate too much from the standard.

Unlike other e-mail parsing libraries that return nested representations of the different MIME parts in a message, this library 
conforms to [RFC 8621, Section 4.1.4](https://datatracker.ietf.org/doc/html/rfc8621#section-4.1.4) and provides a more human-friendly
representation of the message contents consisting of just text body parts, html body parts and attachments. Additionally, conversion to/from
HTML and plain text inline body parts is done automatically when the _alternative_ version is missing.

Performance and memory safety were two important factors while designing _mail-parser_:

- **Zero-copy parsing** is done in most cases (unless when decoding non-UTF8 text or when RFC2047/RFC2231 encoded parts are present). 
  Practically all strings and u8 slices returned by this library are `Cow<str>` or `Cow<[u8]>` references to the input raw message.
- Memory allocations are always avoided unless they are really necessary. In fact, all Base64 and Quoted-Printable parts are decoded in 
  place re-using the input buffer. 
- [Perfect hashing](https://en.wikipedia.org/wiki/Perfect_hash_function) is used for fast look-up of message header fields, character 
  set names and aliases, HTML entities as well as month names while parsing _Date_ fields.
- Although some `unsafe` code was used to obtain performance gains of about 10%, every function in the library has been 
  [fuzzed](#testing-fuzzing--benchmarking) and also heavily [tested with MIRI](#testing-fuzzing--benchmarking).
- Fully battle-tested with millions of real-world e-mail messages dating from 1995 until today.

## Usage Example

```rust
    let mut input = concat!(
        "From: Art Vandelay <art@vandelay.com> (Vandelay Industries)\n",
        "To: \"Colleagues\": \"James Smythe\" <james@vandelay.com>; Friends:\n",
        "    jane@example.com, =?UTF-8?Q?John_Sm=C3=AEth?= <john@example.com>;\n",
        "Date: Sat, 20 Nov 2021 14:22:01 -0800\n",
        "Subject: Why not both importing AND exporting? =?utf-8?b?4pi6?=\n",
        "Content-Type: multipart/mixed; boundary=\"festivus\";\n\n",
        "--festivus\n",
        "Content-Type: text/html; charset=\"us-ascii\"\n",
        "Content-Transfer-Encoding: base64\n\n",
        "PGh0bWw+PHA+SSB3YXMgdGhpbmtpbmcgYWJvdXQgcXVpdHRpbmcgdGhlICZsZHF1bztle\n",
        "HBvcnRpbmcmcmRxdW87IHRvIGZvY3VzIGp1c3Qgb24gdGhlICZsZHF1bztpbXBvcnRpbm\n",
        "cmcmRxdW87LDwvcD48cD5idXQgdGhlbiBJIHRob3VnaHQsIHdoeSBub3QgZG8gYm90aD8\n",
        "gJiN4MjYzQTs8L3A+PC9odG1sPg==\n",
        "--festivus\n",
        "Content-Type: message/rfc822\n\n",
        "From: \"Cosmo Kramer\" <kramer@kramerica.com>\n",
        "Subject: Exporting my book about coffee tables\n",
        "Content-Type: multipart/mixed; boundary=\"giddyup\";\n\n",
        "--giddyup\n",
        "Content-Type: text/plain; charset=\"utf-16\"\n",
        "Content-Transfer-Encoding: quoted-printable\n\n",
        "=FF=FE=0C!5=D8\"=DD5=D8)=DD5=D8-=DD =005=D8*=DD5=D8\"=DD =005=D8\"=\n",
        "=DD5=D85=DD5=D8-=DD5=D8,=DD5=D8/=DD5=D81=DD =005=D8*=DD5=D86=DD =\n",
        "=005=D8=1F=DD5=D8,=DD5=D8,=DD5=D8(=DD =005=D8-=DD5=D8)=DD5=D8\"=\n",
        "=DD5=D8=1E=DD5=D80=DD5=D8\"=DD!=00\n",
        "--giddyup\n",
        "Content-Type: image/gif; name*1=\"about \"; name*0=\"Book \";\n",
        "              name*2*=utf-8''%e2%98%95 tables.gif\n",
        "Content-Transfer-Encoding: Base64\n",
        "Content-Disposition: attachment\n\n",
        "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7\n",
        "--giddyup--\n",
        "--festivus--\n",
    )
    .as_bytes()
    .to_vec();

    let message = Message::parse(&mut input[..]);

    // Parses addresses (including comments), lists and groups
    assert_eq!(
        message.get_from(),
        &Address::Address(Addr {
            name: Some("Art Vandelay (Vandelay Industries)".into()),
            address: Some("art@vandelay.com".into())
        })
    );
    assert_eq!(
        message.get_to(),
        &Address::GroupList(vec![
            Group {
                name: Some("Colleagues".into()),
                addresses: vec![Addr {
                    name: Some("James Smythe".into()),
                    address: Some("james@vandelay.com".into())
                }]
            },
            Group {
                name: Some("Friends".into()),
                addresses: vec![
                    Addr {
                        name: None,
                        address: Some("jane@example.com".into())
                    },
                    Addr {
                        name: Some("John Smîth".into()),
                        address: Some("john@example.com".into())
                    }
                ]
            }
        ])
    );

    assert_eq!(
        message.get_date().unwrap().to_iso8601(),
        "2021-11-20T14:22:01-08:00"
    );

    // RFC2047 support for encoded text in message readers
    assert_eq!(
        message.get_subject().unwrap(),
        "Why not both importing AND exporting? ☺"
    );

    // HTML and text body parts are returned conforming to RFC8621, Section 4.1.4 
    assert_eq!(
        message.get_html_body(0).unwrap().to_string(),
        concat!(
            "<html><p>I was thinking about quitting the &ldquo;exporting&rdquo; to ",
            "focus just on the &ldquo;importing&rdquo;,</p><p>but then I thought,",
            " why not do both? &#x263A;</p></html>"
        )
    );

    // HTML parts are converted to plain text (and viceversa) when missing
    assert_eq!(
        message.get_text_body(0).unwrap().to_string(),
        concat!(
            "I was thinking about quitting the “exporting” to focus just on the",
            " “importing”,\nbut then I thought, why not do both? ☺\n"
        )
    );

    // Supports nested messages as well as multipart/digest
    let nested_message = match message.get_attachment(0).unwrap() {
        MessagePart::Message(v) => v,
        _ => unreachable!(),
    };

    assert_eq!(
        nested_message.get_subject().unwrap(),
        "Exporting my book about coffee tables"
    );

    // Handles UTF-* as well as many legacy encodings
    assert_eq!(
        nested_message.get_text_body(0).unwrap().to_string(),
        "ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!"
    );
    assert_eq!(
        nested_message.get_html_body(0).unwrap().to_string(),
        "<html><body>ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!</body></html>"
    );

    let nested_attachment = match nested_message.get_attachment(0).unwrap() {
        MessagePart::Binary(v) => v,
        _ => unreachable!(),
    };

    assert_eq!(nested_attachment.len(), 42);

    // Full RFC2231 support for continuations and character sets
    assert_eq!(
        nested_attachment
            .get_header()
            .unwrap()
            .get_content_type()
            .unwrap()
            .get_attribute("name")
            .unwrap(),
        "Book about ☕ tables.gif"
    );

    // Integrates with Serde
    println!("{}", serde_json::to_string_pretty(&message).unwrap());
    println!("{}", serde_yaml::to_string(&message).unwrap());
```

## Testing, Fuzzing & Benchmarking

To run the testsuite:

```bash
 $ cargo test --all-features
```

or, to run the testsuite with MIRI:

```bash
 $ cargo +nightly miri test --all-features
```

To fuzz the library with `cargo-fuzz`:

```bash
 $ cargo +nightly fuzz run mail_parser
```

and, to run the benchmarks:

```bash
 $ cargo +nightly bench --all-features
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
- [RFC 2183 - Communicating Presentation Information in Internet Messages: The Content-Disposition Header Field](https://datatracker.ietf.org/doc/html/rfc2183)
- [RFC 6532 - Internationalized Email Headers](https://datatracker.ietf.org/doc/html/rfc6532)
- [RFC 2152 - UTF-7 - A Mail-Safe Transformation Format of Unicode](https://datatracker.ietf.org/doc/html/rfc2152)
- [RFC 2369 - The Use of URLs as Meta-Syntax for Core Mail List Commands and their Transport through Message Header Fields](https://datatracker.ietf.org/doc/html/rfc2369)
- [RFC 2919 - List-Id: A Structured Field and Namespace for the Identification of Mailing Lists](https://datatracker.ietf.org/doc/html/rfc2919)
- [RFC 8621 - The JSON Meta Application Protocol (JMAP) for Mail (Section 4.1.4)](https://datatracker.ietf.org/doc/html/rfc8621#section-4.1.4)

## Supported Character Sets

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
- CP1250 
- CP1251 
- CP1252 
- CP1253 
- CP1254 
- CP1255 
- CP1256 
- CP1257 
- CP1258
- KOI8-R
- KOI8_U
- MACINTOSH
- IBM850
- TIS-620
  
Supported character sets via the optional dependency [encoding_rs](https://crates.io/crates/encoding_rs):
  
- SHIFT_JIS
- BIG5
- EUC-JP 
- EUC-KR 
- GB18030
- GBK
- ISO-2022-JP 
- WINDOWS-874
- IBM-866

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Copyright

Copyright (C) 2020-2022, Stalwart Labs, Minter Ltd.

See [COPYING] for the license.

[COPYING]: https://github.com/stalwartlabs/mail-parser/blob/main/COPYING
