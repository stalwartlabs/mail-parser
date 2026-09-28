/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::{HeaderForm, HeaderName, MessageParser};

const MESSAGE: &[u8] = b"Received: from mx.example.com (mx.example.com [192.0.2.1])\r\n\
\tby mail.example.org with ESMTPS id 4F2A1; Sat, 20 Nov 2021 14:22:05 -0800\r\n\
From: Art Vandelay <art@vandelay.com>\r\n\
To: jane@example.com\r\n\
Subject: Latex imports\r\n\
X-Sender: Kel Varnsen <kel@vandelay.com>\r\n\
X-Tags: latex, import, export\r\n\
X-Mailer: Vandelay Mail 1.0\r\n\
X-Internal-Note: kept, but not parsed\r\n\
\r\n\
Please see the attached catalog.\r\n";

fn main() {
    let parser = MessageParser::new()
        .header(HeaderName::Received, HeaderForm::Raw)
        .header("X-Sender", HeaderForm::Addresses)
        .header("X-Tags", HeaderForm::CommaList)
        .unknown_headers(HeaderForm::Ignore)
        .max_depth(16)
        .max_parts(100);
    let message = parser.parse(MESSAGE).expect("the message parses");

    println!("Header fields with the configured forms:");
    for header in message.headers().iter() {
        println!("  {}: {:?}", header.raw_name(), header.value());
    }

    let received = message
        .headers()
        .get(HeaderName::Received)
        .expect("a Received field");
    let parsed = received.parse_as(HeaderForm::Received);
    if let Some(trace) = parsed.value().as_received() {
        println!(
            "\nReceived, parsed on demand: from {:?} by {:?} with {:?}, id {:?}",
            trace.from(),
            trace.by(),
            trace.with(),
            trace.id()
        );
    }

    let headers_only = MessageParser::new()
        .parse_headers(MESSAGE)
        .expect("the header block parses");
    println!(
        "\nparse_headers: {} header fields, body at byte {}",
        headers_only.headers().len(),
        headers_only.root_part().offset_body()
    );

    println!("\nHeader values parsed on their own:");
    let addresses =
        HeaderForm::Addresses.parse(b" Ann <ann@example.com>, team: bob@example.com;\r\n");
    if let Some(list) = addresses.value().as_address() {
        for mailbox in list.mailboxes() {
            println!(
                "  mailbox {:?} {:?}",
                mailbox.name().unwrap_or_default(),
                mailbox.address().unwrap_or_default()
            );
        }
    }
    let date = HeaderForm::Date.parse(b" Sat, 20 Nov 2021 14:22:01 -0800\r\n");
    if let Some(date) = date.value().as_datetime() {
        println!("  date {} ({})", date.to_rfc3339(), date.to_timestamp());
    }
    let ids = HeaderForm::MessageIds.parse(b" <first@example.com>\r\n <second@example.com>\r\n");
    if let Some(ids) = ids.value().as_text_list() {
        for id in ids.iter() {
            println!("  message id {id}");
        }
    }
    let subject = HeaderForm::Text.parse(b" =?utf-8?q?caf=C3=A9?= au lait\r\n");
    println!("  text {:?}", subject.value().as_text().unwrap_or_default());
}
