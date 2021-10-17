use super::{
    header::{parse_headers, Header},
    message_stream::MessageStream,
};

#[derive(Debug)]
pub struct Message<'x> {
    header: Header<'x>,
}

pub fn parse_message<'x>(stream: &'x MessageStream<'x>) -> Message<'x> {
    let mut message = Message {
        header: Header::new(),
    };

    parse_headers(&mut message.header, stream);

    println!("{:?}", message.header);

    message
}

#[cfg(test)]
mod tests {
    use crate::parsers::message_stream::MessageStream;

    use super::parse_message;

    #[test]
    fn body_parse() {
        let inputs = [(
            concat!(
                "Subject: This is a test email\n",
                "Content-Type: multipart/alternative; boundary=foobar\n",
                "Date: Sun, 02 Oct 2016 07:06:22 -0700 (PDT)\n",
                "Message-Id: <1038776827.1181.6.camel@hurina>\n",
                "List-Id: Dovecot Mailing List <dovecot.procontrol.fi>\n",
                "List-Unsubscribe: <http://procontrol.fi/cgi-bin/mailman/listinfo/dovecot>,\n",
                "    <mailto:dovecot-request@procontrol.fi?subject=unsubscribe>\n",
                "List-Archive: <http://procontrol.fi/pipermail/dovecot>\n",
                "List-Post: <mailto:dovecot@procontrol.fi>\n",
                "List-Help: <mailto:dovecot-request@procontrol.fi?subject=help>\n",
                "List-Subscribe: <http://procontrol.fi/cgi-bin/mailman/listinfo/dovecot>,\n",
                "    <mailto:dovecot-request@procontrol.fi?subject=subscribe>\n",
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
            ),
            "",
        )];

        for input in inputs {
            let value = parse_message(&MessageStream::new(input.0.as_bytes()));
        }
    }
}
