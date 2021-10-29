/*
 * Copyright Stalwart Labs, Minter Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

mod decoders;
mod parsers;

fn main() {
    let mail = concat!(
        "Subject: This is a test email\n",
        "Content-Type: multipart/alternative; boundary=foobar\n",
        "Date: Sun, 02 Oct 2016 07:06:22 -0700 (PDT)\n",
        "\n",
        "--foobar\n",
        "Content-Type: text/plain; charset=utf-8\n",
        "Content-Transfer-Encoding: quoted-printable\n",
        "\n",
        "This is the plaintext version, in utf-8. Proof by Euro: =E2=82=AC\n",
        "--foobar\n",
        "Content-Type: text/html\n",
        "Content-Transfer-Encoding: base64\n",
        "\n",
        "PGh0bWw+PGJvZHk+VGhpcyBpcyB0aGUgPGI+SFRNTDwvYj4gdmVyc2lvbiwgaW4g \n",
        "dXMtYXNjaWkuIFByb29mIGJ5IEV1cm86ICZldXJvOzwvYm9keT48L2h0bWw+Cg== \n",
        "--foobar--\n",
        "After the final boundary stuff gets ignored.\n"
    )
    .to_string();

    //println!("{}", vec![0u8; 100].as_mut_slice().len());

    //let mut parser = MessageStream::new(mail.as_bytes());
}
