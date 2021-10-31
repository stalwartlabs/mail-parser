# mail_parser


## Highlighs
blah
## Usage

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
        "              name*2*=utf-8''%e2%98%95tables.gif\n",
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

    // Supports multipart/digest and nested messages
    let nested_message = match message.get_attachment(0).unwrap() {
        MessagePart::Message(v) => v,
        _ => unreachable!(),
    };

    assert_eq!(
        nested_message.get_subject().unwrap(),
        "Exporting my book about coffee tables"
    );

    // Handles UTF-* as well as many other legacy encodings
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
        "Book about ☕tables.gif"
    );

    // Integrates with Serde
    println!("{}", serde_json::to_string_pretty(&message).unwrap());
    println!("{}", serde_yaml::to_string(&message).unwrap());
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

