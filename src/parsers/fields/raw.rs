use crate::parsers::{header::HeaderValue, message_stream::MessageStream};

pub fn parse_raw<'x>(stream: &'x MessageStream) -> HeaderValue<'x> {
    let mut token_start: usize = 0;
    let mut token_end: usize = 0;
    let mut is_token_safe = true;

    while let Some(ch) = stream.next() {
        match ch {
            b'\n' => match stream.peek() {
                Some(b' ' | b'\t') => {
                    stream.advance(1);
                    continue;
                }
                _ => {
                    return if token_start > 0 {
                        HeaderValue::String(
                            stream
                                .get_string(token_start - 1, token_end, is_token_safe)
                                .unwrap(),
                        )
                    } else {
                        HeaderValue::Empty
                    };
                }
            },
            b' ' | b'\t' | b'\r' => continue,
            0..=0x7f => (),
            _ => {
                if is_token_safe {
                    is_token_safe = false;
                }
            }
        }

        if token_start == 0 {
            token_start = stream.get_pos();
        }

        token_end = stream.get_pos();
    }

    HeaderValue::Empty
}

mod tests {
    use crate::parsers::{fields::raw::parse_raw, header::HeaderValue, message_stream::MessageStream};

    #[test]
    fn parse_raw_text() {
        let inputs = [
            ("Saying Hello\nMessage-Id", "Saying Hello"),
            ("Re: Saying Hello\r\n \r\nFrom:", "Re: Saying Hello"),
            (
                concat!(
                    " from x.y.test\n      by example.net\n      via TCP\n",
                    "      with ESMTP\n      id ABC12345\n      ",
                    "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600\n"
                ),
                concat!(
                    "from x.y.test\n      by example.net\n      via TCP\n",
                    "      with ESMTP\n      id ABC12345\n      ",
                    "for <mary@example.net>;  21 Nov 1997 10:05:43 -0600"
                ),
            ),
        ];

        for input in inputs {
            assert_eq!(
                parse_raw(&MessageStream::new(input.0.as_bytes())),
                HeaderValue::String(input.1.into()),
                "Failed for '{:?}'",
                input.0
            );
        }
    }
}
