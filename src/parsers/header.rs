use crate::parsers::message_stream::MessageStream;
use crate::parsers::address::*;
pub enum Header<'x> {
    Sender(Address<'x>),
    From(Address<'x>),
    To(AddressList<'x>),
    Cc(AddressList<'x>),
    Bcc(AddressList<'x>),
    Subject(&'x str),
    Other((&'x str, &'x str)),
}

/*

List-Help
List-Unsubscribe
List-Subscribe
List-Post
List-Owner
List-Archive

*/

pub enum EncodingType {
    QuotedPrintable,
    Base64,
    Bit7,
    Bit8
}

pub struct NonAsciiText<'x> {
    encoding: EncodingType,
    charset: &'x [u8],
    data: &'x [u8]

}


impl<'x> Header<'x> {



    pub fn parse(s: &'x mut MessageStream) -> Option<Header<'x>> {
        let mut ch_hash_1: u8 = 0;
        let mut ch_hash_2: u8 = 0;
        let mut start_pos: isize = -1;
        let mut end_pos: isize = -1;

        let hash_ch  = |x: u8| -> u32 {
            match x {
                98 | 99 | 101 | 102 | 105 | 114 => 0,
                103 | 104 | 107 => 5,
                100 | 116 => 10,
                109 | 115 => 15,
                111 => 25,
                _ => 38
            }
        };

        while let Some(ch) = (*s).get_lower_char() {
            match ch {
                10 => return None,
                13 | 9 | 32 => (),         // CR TAB SPACE
                58 if start_pos != -1 => { // :
                    let field_len: u32 = (end_pos - start_pos + 1) as u32;

                    if field_len >= 2 && field_len <= 25 {
                        let hash = field_len + hash_ch(ch_hash_2) + hash_ch(ch_hash_1);

                        if hash >= 2 && hash <= 37 {
                            let field = (*s).get_bytes(start_pos as usize, (end_pos + 1) as usize).unwrap();

                            match hash - 2 {
                                0 if field == b"cc" => {
                                },
                                1 if field == b"bcc" => {
                                },
                                7 if field == b"resent-cc" => {
                                },
                                8 if field == b"resent-bcc" => {
                                },
                                9 if field == b"resent-date" => {
                                },
                                10 if field == b"content-type" => {
                                },
                                11 if field == b"resent-sender" => {
                                },
                                12 if field == b"date" => {
                                },
                                14 if field == b"return-path" => {
                                },
                                16 if field == b"received" => {
                                },
                                17 if field == b"from" => {
                                },
                                18 if field == b"content-id" => {
                                },
                                19 if field == b"sender" => {
                                },
                                21 if field == b"comments" => {
                                },
                                23 if field == b"references" => {
                                },
                                24 if field == b"resent-from" => {
                                },
                                25 if field == b"resent-message-id" => {
                                },
                                26 if field == b"keywords" => {
                                },
                                28 if field == b"content-transfer-encoding" => {
                                },
                                30 if field == b"subject" => {
                                    println!("Subject!");
                                },
                                31 if field == b"reply-to" => {
                                },
                                32 if field == b"resent-to" => {
                                },
                                33 if field == b"message-id" => {
                                },
                                34 if field == b"in-reply-to" => {
                                },
                                35 if field == b"to" => {
                                },
                                _ => ()
                            }
                        }
                    }
                },
                _ => {
                    if start_pos != -1 {
                        end_pos = (*s).get_read_pos();
                        ch_hash_2 = ch;
                    } else {
                        start_pos = (*s).get_read_pos();
                        end_pos = start_pos;
                        ch_hash_1 = ch;
                    }
                }
            }
        }
        None
    }
}
