use std::borrow::Cow;

use super::CharsetDecoder;

pub struct Utf8Decoder<'x> {
    result: Option<Cow<'x, str>>
}

impl<'x> CharsetDecoder for Utf8Decoder<'x> {
    fn ingest(&mut self, ch: u8) -> () {
        panic!("Slice needed!")
    }

    fn ingest_slice(&mut self, chs: &[u8]) -> () {
//        self.result = Some(String::from_utf8_lossy(chs));
    }

    fn to_string(&self) -> Option<&str> {
        None
        /*match self.result {
            Some(result) => {
                Some(result.as_ref())
            },
            None => None
        }*/
    }

    fn needs_slice(&self) -> bool {
        true
    }
}

impl<'x> Utf8Decoder<'x> {
    pub fn new() -> Utf8Decoder<'x> {
        Utf8Decoder {
            result: None
        }
    }

    fn to_string2(&'x self) -> Option<&'x str> {
        match &self.result {
            Some(result) => {
                Some(Cow::Borrowed(&result).as_ref())
            },
            None => None
        }
    }

    fn ingest_slice2(&'x mut self, chs: &'x [u8]) -> () {
        self.result = Some(String::from_utf8_lossy(chs));
    }    
}
