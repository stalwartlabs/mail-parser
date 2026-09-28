/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{Address, Mailbox, MessageParser, MessagePart, PartKind};

const MESSAGE: &[u8] = br#"From: Art Vandelay <art@vandelay.com> (Vandelay Industries)
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

fn main() {
    let message = MessageParser::new()
        .parse(MESSAGE)
        .expect("the message parses");

    println!("Subject: {}", message.subject().unwrap_or_default());
    if let Some(date) = message.date() {
        println!("Date: {}", date.to_rfc3339());
    }
    for sender in message.from().into_iter().flat_map(|from| from.mailboxes()) {
        println!("From: {}", display(sender));
    }
    for recipient in message.to().into_iter().flat_map(|to| to.iter()) {
        match recipient {
            Address::Mailbox(mailbox) => println!("To: {}", display(mailbox)),
            Address::Group(group) => {
                println!("To group {}:", group.name().unwrap_or_default());
                for member in group.mailboxes() {
                    println!("    {}", display(member));
                }
            }
        }
    }

    println!("\nHeader fields as written, with their parsed values:");
    for header in message.headers().iter() {
        println!("  {}: {:?}", header.raw_name(), header.value());
    }

    println!("\nText body (HTML converted to text):");
    for text in message.text_bodies() {
        println!("{text}");
    }
    println!("\nHTML body:");
    for html in message.html_bodies() {
        println!("{html}");
    }

    println!("\nEvery part of every message, in document order:");
    for part in message.parts() {
        describe(part);
    }

    if let Some(multipart) = message.part_by_boundary("giddyup") {
        println!(
            "\nThe boundary \"giddyup\" belongs to part {}, which has {} children",
            multipart.id(),
            multipart.children().len()
        );
    }

    for attachment in message.attachments() {
        let PartKind::Message(nested) = attachment.kind() else {
            continue;
        };
        println!(
            "\nNested message {}: {}",
            nested.id(),
            nested.subject().unwrap_or_default()
        );
        for text in nested.text_bodies() {
            println!("  text: {text}");
        }
        for file in nested.attachments() {
            println!(
                "  attachment {:?}: {} bytes",
                file.attachment_name().unwrap_or_default(),
                file.decoded_len()
            );
        }
    }
}

fn display(mailbox: Mailbox<'_>) -> String {
    let address = mailbox.address().unwrap_or_default();
    match mailbox.name() {
        Some(name) => format!("{name} <{address}>"),
        None => address.to_string(),
    }
}

fn describe(part: MessagePart<'_>) {
    let content_type = part
        .content_type()
        .map(|content_type| {
            format!(
                "{}/{}",
                content_type.ctype(),
                content_type.subtype().unwrap_or_default()
            )
        })
        .unwrap_or_else(|| "text/plain".to_string());
    let summary = match part.kind() {
        PartKind::Multipart => format!(
            "boundary {:?}, {} children",
            part.boundary().unwrap_or_default(),
            part.children().len()
        ),
        PartKind::Message(nested) => format!("holds message {}", nested.id()),
        PartKind::Text | PartKind::Html => match part.text_checked() {
            Some((text, problems)) if problems.is_empty() => {
                format!("{} characters", text.chars().count())
            }
            Some((text, problems)) => format!(
                "{} characters, decoding problems: {problems:?}",
                text.chars().count()
            ),
            None => String::new(),
        },
        PartKind::Binary | PartKind::InlineBinary => format!(
            "{} bytes as written, {} decoded",
            part.raw_body().len(),
            part.decoded_len()
        ),
    };
    println!(
        "  part {} of message {} ({content_type}, {:?}): {summary}",
        part.id(),
        part.message().id(),
        part.role()
    );
}
