/*
 *  assert equal
 */

fn main() {
    assert_eq!("string", r#"string"#);
    assert_eq!(b"bytes", br#"bytes"#);

    let input = concat!(
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
    .as_bytes();

    let input_rs = br#"From: Art Vandelay <art@vandelay.com> (Vandelay Industries)
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

    assert_eq!(input, input_rs);
}
