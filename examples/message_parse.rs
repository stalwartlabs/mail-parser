/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use mail_parser::*;

fn main() {
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

    let message = MessageParser::default().parse(input).unwrap();

    // Parses addresses (including comments), lists and groups
    assert_eq!(
        message.from().unwrap().first().unwrap(),
        &Addr::new(
            "Art Vandelay (Vandelay Industries)".into(),
            "art@vandelay.com"
        )
    );

    assert_eq!(
        message.to().unwrap().as_group().unwrap(),
        &[
            Group::new(
                "Colleagues",
                vec![Addr::new("James Smythe".into(), "james@vandelay.com")]
            ),
            Group::new(
                "Friends",
                vec![
                    Addr::new(None, "jane@example.com"),
                    Addr::new("John Smîth".into(), "john@example.com"),
                ]
            )
        ]
    );

    assert_eq!(
        message.date().unwrap().to_rfc3339(),
        "2021-11-20T14:22:01-08:00"
    );

    // RFC2047 support for encoded text in message readers
    assert_eq!(
        message.subject().unwrap(),
        "Why not both importing AND exporting? ☺"
    );

    // HTML and text body parts are returned conforming to RFC8621, Section 4.1.4
    assert_eq!(
        message.body_html(0).unwrap(),
        concat!(
            "<html><p>I was thinking about quitting the &ldquo;exporting&rdquo; to ",
            "focus just on the &ldquo;importing&rdquo;,</p><p>but then I thought,",
            " why not do both? &#x263A;</p></html>"
        )
    );

    // HTML parts are converted to plain text (and viceversa) when missing
    assert_eq!(
        message.body_text(0).unwrap(),
        concat!(
            "I was thinking about quitting the “exporting” to focus just on the",
            " “importing”,\nbut then I thought, why not do both? ☺\n"
        )
    );

    // Supports nested messages as well as multipart/digest
    let nested_message = message.attachment(0).unwrap().message().unwrap();

    assert_eq!(
        nested_message.subject().unwrap(),
        "Exporting my book about coffee tables"
    );

    // Handles UTF-* as well as many legacy encodings
    assert_eq!(
        nested_message.body_text(0).unwrap(),
        "ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!"
    );
    assert_eq!(
        nested_message.body_html(0).unwrap(),
        "<html><body>ℌ𝔢𝔩𝔭 𝔪𝔢 𝔢𝔵𝔭𝔬𝔯𝔱 𝔪𝔶 𝔟𝔬𝔬𝔨 𝔭𝔩𝔢𝔞𝔰𝔢!</body></html>"
    );

    let nested_attachment = nested_message.attachment(0).unwrap();

    assert_eq!(nested_attachment.len(), 42);

    // Full RFC2231 support for continuations and character sets
    assert_eq!(
        nested_attachment.attachment_name().unwrap(),
        "Book about ☕ tables.gif"
    );

    // Integrates with Serde
    println!("{}", serde_json::to_string_pretty(&message).unwrap());
}
