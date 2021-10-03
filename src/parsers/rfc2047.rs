use crate::parsers::message_stream::MessageStream;

enum Rfc2047State {
    Charset,
    Language,
    Encoding,
    Data
}

enum Rfc2047Encoding {
    Invalid,
    Base64,
    QuotedPrintable
}

pub fn try_decode<'x>(stream: &'x mut MessageStream) -> Option<&'x str> {
    let mut charset: [u8; 10] = [0; 10];
    let mut state = Rfc2047State::Charset;
    let mut encoding = Rfc2047Encoding::Invalid;
    let mut pos: usize = 0;

    for mut ch in stream.by_ref() {
        match state {
            Rfc2047State::Charset => {
                match ch {
                    65..=90 => { // A-Z
                        ch = ch + 32; // To uppercase
                    },
                    97..=122 | 45 | 95 => (), // a-z - _
                    63 => { // ?
                        if pos > 0 {
                            println!("Charset: {}", unsafe { std::str::from_utf8_unchecked(&charset[0..pos]) });
                            state = Rfc2047State::Encoding;
                            continue;
                        } else {
                            return None;
                        }
                    },
                    42 => { // *
                        state = Rfc2047State::Language;
                        continue;
                    },
                    _ => {
                        //stream.rewind(1);
                        return None;
                    }
                }

                // Add to charset array
                match charset.get_mut(pos) {
                    Some(val) => {
                        *val = ch;
                    },
                    None => {return None;}
                }
                pos = pos + 1;
            },
            Rfc2047State::Language => { // Ignore language
                match ch {
                    65..=90 | 97..=122 | 45 | 95 => (),
                    63 => { // ?
                        if pos > 0 {
                            println!("Charset: {}", unsafe { std::str::from_utf8_unchecked(&charset[0..pos]) });
                            state = Rfc2047State::Encoding;
                            continue;
                        } else {
                            return None;
                        }
                    },
                    _ => {
                        return None;
                    }                    
                }
            },
            Rfc2047State::Encoding => {
                match ch {
                    113 | 81 => { // q Q
                        if let Rfc2047Encoding::Invalid = encoding {
                            encoding = Rfc2047Encoding::QuotedPrintable;
                        } else {
                            return None;
                        }
                    },
                    98 | 66 => { // b B
                        if let Rfc2047Encoding::Invalid = encoding {
                            encoding = Rfc2047Encoding::Base64;
                        } else {
                            return None;
                        }
                    },
                    63 => { // ?
                        if let Rfc2047Encoding::Invalid = encoding {
                            return None;
                        }
                        state = Rfc2047State::Data;
                    },
                    _ => {
                        return None;
                    }
                }
            },
            Rfc2047State::Data => {
                match ch {
                    63 => {            // ?

                    }, 
                    13 | 9 | 32 => (), // CR TAB SPACE
                    10 => {            // LF
                        return None;
                    },
                    _ => ()
                }
            }
        }
    }

    None
}
