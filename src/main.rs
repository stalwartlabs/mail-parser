use std::{
    borrow::{BorrowMut, Cow},
    error::Error
};

/*

"Date:" date-time CRLF
"From:" mailbox-list CRLF
"Sender:" mailbox CRLF
"Reply-To:" address-list CRLF
"To:" address-list CRLF
"Cc:" address-list CRLF
"Bcc:" [address-list / CFWS] CRLF
"Message-ID:" msg-id CRLF
"In-Reply-To:" 1*msg-id CRLF
"References:" 1*msg-id CRLF
"Subject:" unstructured CRLF
"Comments:" unstructured CRLF
"Keywords:" phrase *("," phrase) CRLF
"Resent-Date:" date-time CRLF
"Resent-From:" mailbox-list CRLF
"Resent-Sender:" mailbox CRLF
"Resent-To:" address-list CRLF
"Resent-Cc:" address-list CRLF
"Resent-Bcc:" [address-list / CFWS] CRLF
"Resent-Message-ID:" msg-id CRLF
"Return-Path:" path CRLF
"Received:" *received-token ";" date-time CRLF
Content-Type: multipart/mixed; boundary=gc0p4Jq0M2Yt08j34c0p
Content-transfer-encoding: base64
"Content-ID" ":" msg-id



*/

mod parsers;
mod decoders;



/*struct Message<'x> {
    headers: Vec<Header<'x>>,
}*/
use crate::parsers::message_stream::MessageStream;
use crate::parsers::header::Header;

static HEX_TO_VAL: &'static [i8] = &[
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 0,  1,  2,  3,  4,  5,  6,  7,  8,
    9,  -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, 10, 11, 12, 13, 14, 15, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1];


fn main() {
    let mut mail = concat!(
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

    //let mut parser = MessageStream::new(unsafe { mail.as_bytes_mut() });

    //let hdr = Header::parse(&mut parser);

    let val = (HEX_TO_VAL['3' as usize] << 4) as u8 | HEX_TO_VAL['f' as usize] as u8;

    println!("{}", val);


    /*static HASH_VALUES: &'static [i8] = 
    &[38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38,  0,  0,
    10,  0,  0,  5,  5,  0, 38,  5, 38, 15,
    38, 25, 38, 38,  0, 15, 10, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38, 38, 38, 38, 38,
    38, 38, 38, 38, 38, 38];

    for item in HASH_VALUES.into_iter().enumerate() {
        let (i, x): (usize, &i8) = item;
        if *x != 38 {
            println!("{} => {},", i, x);

        }
    }*/

    //let part1 = std::str::from_utf8(parser.get_slice(0, 10)).unwrap();
    //let part2 = std::str::from_utf8(parser.get_slice(11, 20)).unwrap();

    //println!("{}-{}", part1, part2 );
}

/*
pub fn qp_decode<'x>(stream: &'x mut MessageStream, is_header: bool) -> Option<&'x [u8]> {
    let mut state: QuotedPrintableState = QuotedPrintableState::None;
    let mut hex_1: i8 = 0;
    let output_start = stream.begin_write();

    while let Some(ch) = stream.next() {
        match ch {
            b'=' => {  // =
                if let QuotedPrintableState::None = state {
                    state = QuotedPrintableState::Eq;
                } else {
                    break;
                }
            },
            b'\n' if !is_header => { // \n
                match state {
                    QuotedPrintableState::Eq => {
                        state = QuotedPrintableState::None;
                    },
                    QuotedPrintableState::Hex1 => { 
                        break; 
                    },
                    QuotedPrintableState::None => {
                        stream.write(b'\n');
                    }
                }
            },
            b'\n' if is_header => {
                break;
            },
            b'\r' | b'\t' => (), // \r
            b'_' if is_header => {
                match state {
                    QuotedPrintableState::None => {
                        stream.write(b' ');
                    },
                    _ => {
                        break;
                    }
                }
            },
            b' '..=b'~' => {
                match state {
                    QuotedPrintableState::None => {
                        stream.write(ch);
                    },
                    QuotedPrintableState::Eq => {
                        hex_1 = unsafe{*HEX_TO_VAL.get_unchecked(ch as usize)};
                        if hex_1 == -1 {
                            break;
                        } else {
                            state = QuotedPrintableState::Hex1;
                        }
                    },
                    QuotedPrintableState::Hex1 => {
                        let hex2 = unsafe{*HEX_TO_VAL.get_unchecked(ch as usize)};
                        
                        state = QuotedPrintableState::None;
                        if hex2 == -1 {
                            break;
                        } else {
                            stream.write(((hex_1 as u8) << 4) | hex2 as u8);
                        }
                    }
                }
            },
            _ => {
                if let QuotedPrintableState::None = state {
                    stream.write(b'!');
                } else {
                    break;
                }
            }
        }
    }

    let output_end = stream.get_write_pos();
    if output_end > output_start {
        return stream.get_bytes(output_start, output_end);
    } else {
        return None;
    }
}
*/