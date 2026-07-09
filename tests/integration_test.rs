/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use mail_parser::*;

#[test]
fn test_api() {
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

    // Default parser
    let message = MessageParser::default().parse(input).unwrap();
    let headers = MessageParser::default().parse_headers(input).unwrap();
    let custom_message = MessageParser::default()
        .with_minimal_headers()
        .parse(input)
        .unwrap();

    assert_eq!(message.headers(), headers.headers());
    assert_eq!(message.headers(), custom_message.headers());
    assert_eq!(message.parts.len(), 3);
    assert_eq!(headers.parts.len(), 1);
    assert_eq!(message.parts.len(), custom_message.parts.len());
    assert_eq!(message.parts, custom_message.parts);

    assert_eq!(
        bincode::deserialize::<Vec<Header>>(
            &bincode::serialize(&message.parts[0].headers).unwrap()
        )
        .unwrap(),
        message.parts[0].headers
    );

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

    assert_eq!(
        message.subject().unwrap(),
        "Why not both importing AND exporting? ☺"
    );

    assert_eq!(
        message.body_html(0).unwrap(),
        concat!(
            "<html><p>I was thinking about quitting the &ldquo;exporting&rdquo; to ",
            "focus just on the &ldquo;importing&rdquo;,</p><p>but then I thought,",
            " why not do both? &#x263A;</p></html>"
        )
    );

    assert_eq!(
        message.body_text(0).unwrap(),
        concat!(
            "I was thinking about quitting the “exporting” to focus just on the",
            " “importing”,\nbut then I thought, why not do both? ☺\n"
        )
    );

    let nested_message = message.attachment(0).unwrap().message().unwrap();

    assert_eq!(
        nested_message.subject().unwrap(),
        "Exporting my book about coffee tables"
    );

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

    assert_eq!(
        nested_attachment.attachment_name().unwrap(),
        "Book about ☕ tables.gif"
    );
}

#[cfg(feature = "cli")]
mod test_cli {
    use std::{
        fs,
        io::{stdin, IsTerminal, Write},
        path::Path,
        process::{Command, Stdio},
    };

    use serde_json::Value;

    const BIN: &str = env!("CARGO_BIN_EXE_mail-parser");

    fn fixture(name: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/eml/rfc")
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    fn run_with_stdin(args: &[&str], input: Vec<u8>) -> (bool, String) {
        let mut child = Command::new(BIN)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn process");

        child
            .stdin
            .take()
            .unwrap()
            .write_all(&input)
            .expect("failed to write stdin");

        let out = child.wait_with_output().expect("failed to wait");

        let success = out.status.success();
        let out = String::from_utf8_lossy(&out.stdout).into_owned();

        (success, out)
    }

    /// Run the binary with args, inheriting the test process's stdin.
    ///
    /// The path and inline modes require stdin to be a terminal so
    /// the binary can distinguish between "message provided as arg"
    /// and "read from pipe".  Returns `None` when the test process
    /// itself has no terminal stdin (e.g. in CI), allowing those
    /// tests to be skipped gracefully.
    fn run_with_tty_stdin(args: &[&str]) -> Option<(bool, String)> {
        if !stdin().is_terminal() {
            return None;
        }

        let out = Command::new(BIN)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("failed to run process");

        let success = out.status.success();
        let out = String::from_utf8_lossy(&out.stdout).into_owned();

        Some((success, out))
    }

    fn subject_of(json: &Value) -> &str {
        json["parts"][0]["headers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|h| h["name"] == "subject")
            .expect("subject header not found")["value"]["Text"]
            .as_str()
            .unwrap()
    }

    #[test]
    fn message_as_stdin() {
        let eml = fs::read(fixture("001.eml")).expect("failed to read fixture");
        let (ok, stdout) = run_with_stdin(&[], eml);
        assert!(ok, "process did not exit successfully");

        let json: Value = serde_json::from_str(&stdout).expect("stdout is not valid JSON");
        assert_eq!(subject_of(&json), "whatever");
    }

    #[test]
    fn message_as_path() {
        let Some((ok, stdout)) = run_with_tty_stdin(&[&fixture("001.eml")]) else {
            eprintln!("skipped: no terminal stdin");
            return;
        };
        assert!(ok, "process did not exit successfully");

        let json: Value = serde_json::from_str(&stdout).expect("stdout is not valid JSON");
        assert_eq!(subject_of(&json), "whatever");
    }

    #[test]
    fn inline_message() {
        let msg = "From: Art Vandelay <art@vandelay.com>\nSubject: Inline test\n\nHello";
        let Some((ok, stdout)) = run_with_tty_stdin(&[msg]) else {
            eprintln!("skipped: no terminal stdin");
            return;
        };
        assert!(ok, "process did not exit successfully");

        let json: Value = serde_json::from_str(&stdout).expect("stdout is not valid JSON");
        assert_eq!(subject_of(&json), "Inline test");
    }

    #[test]
    fn pretty_flag() {
        let eml = fs::read(fixture("001.eml")).expect("failed to read fixture");

        let (ok_compact, compact) = run_with_stdin(&[], eml.clone());
        let (ok_pretty, pretty) = run_with_stdin(&["--pretty"], eml);

        assert!(ok_compact && ok_pretty, "process did not exit successfully");

        assert_eq!(
            compact.lines().count(),
            1,
            "compact output must be a single line"
        );

        assert!(
            pretty.lines().count() > 1,
            "pretty output must span multiple lines"
        );

        let a: Value = serde_json::from_str(&compact).expect("compact is not valid JSON");
        let b: Value = serde_json::from_str(&pretty).expect("pretty is not valid JSON");

        assert_eq!(a, b, "compact and pretty must represent the same data");
    }
}
