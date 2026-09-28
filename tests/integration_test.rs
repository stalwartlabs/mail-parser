/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

mod support;

use mail_parser::{Address, MessageParser, PartKind};
use support::regression;

const README_EXAMPLE: &str = "readme-example";

#[test]
fn test_api() {
    let raw = regression(README_EXAMPLE);
    let message = MessageParser::default()
        .parse(&raw)
        .expect("the message parses");
    let headers = MessageParser::default()
        .parse_headers(&raw)
        .expect("the headers parse");

    assert_eq!(message.headers().len(), headers.headers().len());
    for (full, only) in message.headers().iter().zip(headers.headers().iter()) {
        assert_eq!(full.name(), only.name());
        assert_eq!(full.raw_value(), only.raw_value());
        assert_eq!(full.offset_start(), only.offset_start());
    }
    assert_eq!(message.messages().len(), 2);
    assert_eq!(message.parts().len(), 6);
    assert_eq!(headers.parts().len(), 1);

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
            "I was thinking about quitting the \u{201c}exporting\u{201d} to focus just on the",
            " \u{201c}importing\u{201d},\nbut then I thought, why not do both? \u{263a}\n"
        ))
    );

    let container = message.attachments().next().expect("attachment");
    let PartKind::Message(nested) = container.kind() else {
        panic!("expected a nested message");
    };
    assert_eq!(nested.id(), 1);
    assert_eq!(
        nested.body_text(0).as_deref(),
        Some(
            "\u{210c}\u{1d522}\u{1d529}\u{1d52d} \u{1d52a}\u{1d522} \u{1d522}\u{1d535}\u{1d52d}\u{1d52c}\u{1d52f}\u{1d531} \u{1d52a}\u{1d536} \u{1d51f}\u{1d52c}\u{1d52c}\u{1d528} \u{1d52d}\u{1d529}\u{1d522}\u{1d51e}\u{1d530}\u{1d522}!"
        )
    );
    assert_eq!(
        nested.body_html(0).as_deref(),
        Some(
            "<html><body>\u{210c}\u{1d522}\u{1d529}\u{1d52d} \u{1d52a}\u{1d522} \u{1d522}\u{1d535}\u{1d52d}\u{1d52c}\u{1d52f}\u{1d531} \u{1d52a}\u{1d536} \u{1d51f}\u{1d52c}\u{1d52c}\u{1d528} \u{1d52d}\u{1d529}\u{1d522}\u{1d51e}\u{1d530}\u{1d522}!</body></html>"
        )
    );

    let nested_attachment = nested.attachments().next().expect("nested attachment");
    assert_eq!(nested_attachment.decoded_len(), 42);
    assert_eq!(nested_attachment.decoded().len(), 42);
    assert_eq!(
        nested_attachment.attachment_name(),
        Some("Book about \u{2615} tables.gif")
    );
}

#[cfg(all(feature = "serde", feature = "full_encoding"))]
mod full_messages {
    use crate::support::{snapshot_inputs, snapshot_messages};
    use mail_parser::MessageParser;
    use std::fs;

    #[test]
    fn parse_full_messages() {
        let parser = MessageParser::new();
        let mut failed = Vec::new();
        for path in snapshot_messages() {
            let raw = fs::read(&path).expect("the message is readable");
            for (input, extension) in snapshot_inputs(&raw) {
                let json = serde_json::to_string_pretty(&parser.parse(&input))
                    .expect("the message serializes");
                let expected = path.with_extension(extension);
                if fs::read(&expected).ok().as_deref() != Some(json.as_bytes()) {
                    let output = expected.with_extension("failed");
                    fs::write(&output, &json).expect("the output is writable");
                    failed.push(output.display().to_string());
                }
            }
        }
        assert!(
            failed.is_empty(),
            "{} parses differ from their fixture; the parsed messages were saved to:\n{}",
            failed.len(),
            failed.join("\n")
        );
    }
}

#[test]
fn test_api_field_parsers() {
    let raw = regression(README_EXAMPLE);
    let message = MessageParser::default()
        .parse(&raw)
        .expect("the message parses");

    let from = message.from().and_then(|from| from.first()).expect("from");
    assert_eq!(from.name(), Some("Art Vandelay (Vandelay Industries)"));
    assert_eq!(from.address(), Some("art@vandelay.com"));

    let groups: Vec<_> = message
        .to()
        .expect("to")
        .iter()
        .map(|address| match address {
            Address::Group(group) => (
                group.name().map(str::to_string),
                group
                    .mailboxes()
                    .map(|mailbox| {
                        (
                            mailbox.name().map(str::to_string),
                            mailbox.address().map(str::to_string),
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
            Address::Mailbox(_) => panic!("expected groups"),
        })
        .collect();
    assert_eq!(
        groups,
        [
            (
                Some("Colleagues".to_string()),
                vec![(
                    Some("James Smythe".to_string()),
                    Some("james@vandelay.com".to_string())
                )]
            ),
            (
                Some("Friends".to_string()),
                vec![
                    (None, Some("jane@example.com".to_string())),
                    (
                        Some("John Sm\u{ee}th".to_string()),
                        Some("john@example.com".to_string())
                    ),
                ]
            ),
        ]
    );
    assert_eq!(
        message.date().map(|date| date.to_rfc3339()).as_deref(),
        Some("2021-11-20T14:22:01-08:00")
    );
    assert_eq!(
        message.subject(),
        Some("Why not both importing AND exporting? \u{263a}")
    );
    let container = message.attachments().next().expect("attachment");
    assert_eq!(
        container.nested().and_then(|nested| nested.subject()),
        Some("Exporting my book about coffee tables")
    );
}
