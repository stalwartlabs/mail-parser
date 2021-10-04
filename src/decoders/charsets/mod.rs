pub mod parser;
pub mod single_byte;
#[cfg(feature = "multibytedecode")]
pub mod multi_byte;
pub mod utf8;

pub trait CharsetDecoder {
    fn ingest(&mut self, ch: u8) -> ();
    fn ingest_slice(&mut self, chs: &[u8]) -> () {
        for ch in chs {
            self.ingest(*ch);
        }
    }
    fn needs_slice(&self) -> bool {
        false
    }
    fn to_string(&self) -> Option<&str>;
}

#[cfg(test)]
mod tests {
    use super::parser::CharsetParser;

    #[test]
    fn decode() {
        let mut parser = CharsetParser::new();
        let inputs = [
            ("iso-8859-1".as_bytes(), b"\xe1\xe9\xed\xf3\xfa".to_vec(), "áéíóú"),
            ("iso-8859-5".as_bytes(), b"\xbf\xe0\xd8\xd2\xd5\xe2, \xdc\xd8\xe0".to_vec(), "Привет, мир"),
            ("iso-8859-6".as_bytes(), b"\xe5\xd1\xcd\xc8\xc7 \xc8\xc7\xe4\xd9\xc7\xe4\xe5".to_vec(),"مرحبا بالعالم"),
            ("iso-8859-7".as_bytes(), b"\xc3\xe5\xe9\xdc \xf3\xef\xf5 \xca\xfc\xf3\xec\xe5".to_vec(),"Γειά σου Κόσμε"),
            ("iso-8859-8".as_bytes(), b"\xf9\xec\xe5\xed \xf2\xe5\xec\xed".to_vec(),"שלום עולם"),
            ("iso-8859-11".as_bytes(), b"\xc3\xcb\xd1\xca\xca\xd3\xcb\xc3\xd1\xba\xcd\xd1\xa1\xa2\xc3\xd0\xe4\xb7\xc2\xb7\xd5\xe8\xe3\xaa\xe9\xa1\xd1\xba\xa4\xcd\xc1\xbe\xd4\xc7\xe0\xb5\xcd\xc3\xec".to_vec(),"รหัสสำหรับอักขระไทยที่ใช้กับคอมพิวเตอร์"),
            ("windows-1250".as_bytes(), b"Zelo rada grem v sla\x9a\xe8i\xe8arno".to_vec(),"Zelo rada grem v slaščičarno"),
            ("windows-1251".as_bytes(), b"\xcf\xf0\xe8\xe2\xe5\xf2, \xec\xe8\xf0".to_vec(),"Привет, мир"),
            ("windows-1252".as_bytes(), b"\xa1El \xf1and\xfa comi\xf3 \xf1oquis!".to_vec(),"¡El ñandú comió ñoquis!"),
            ("windows-1253".as_bytes(), b"\xca\xf9\xe4\xe9\xea\xef\xdf \xd3\xf9\xea\xf1\xdc\xf4\xe7\xf2 \xf3\xf4\xef Rust".to_vec(),"Κωδικοί Σωκράτης στο Rust"),
            ("windows-1254".as_bytes(), b"Kebab\xfdm\xfd baharatl\xfd yapma".to_vec(),"Kebabımı baharatlı yapma"),
            ("windows-1255".as_bytes(), b"\xf9\xec\xe5\xed \xf2\xe5\xec\xed".to_vec(),"שלום עולם"),
            ("windows-1256".as_bytes(), b"\xe3\xd1\xcd\xc8\xc7 \xc8\xc7\xe1\xda\xc7\xe1\xe3".to_vec(),"مرحبا بالعالم"),
            ("windows-1257".as_bytes(), b"Mu h\xf5ljuk on angerjaid t\xe4is".to_vec(),"Mu hõljuk on angerjaid täis"),
            ("windows-1258".as_bytes(), b"Xin ch\xe0o".to_vec(),"Xin chào"),
            ("macintosh".as_bytes(), b"\x87\x8e\x92\x97\x9c".to_vec(),"áéíóú"),
            ("ibm850".as_bytes(), b"\x9b\x9c\x9d\x9e".to_vec(),"ø£Ø×"),
            ("koi8-r".as_bytes(), b"\xf0\xd2\xc9\xd7\xc5\xd4, \xcd\xc9\xd2".to_vec(),"Привет, мир"),
            ("koi8-u".as_bytes(), b"\xf0\xd2\xc9\xd7\xa6\xd4 \xf3\xd7\xa6\xd4".to_vec(),"Привіт Світ"),

            #[cfg(feature = "multibytedecode")]
            ("shift_jis".as_bytes(), b"\x83n\x83\x8D\x81[\x81E\x83\x8F\x81[\x83\x8B\x83h".to_vec(),"ハロー・ワールド"),
            #[cfg(feature = "multibytedecode")]
            ("big5".as_bytes(), b"\xa7A\xa6n\xa1A\xa5@\xac\xc9".to_vec(),"你好，世界"),
            #[cfg(feature = "multibytedecode")]
            ("euc-jp".as_bytes(), b"\xa5\xcf\xa5\xed\xa1\xbc\xa1\xa6\xa5\xef\xa1\xbc\xa5\xeb\xa5\xc9".to_vec(),"ハロー・ワールド"),
            #[cfg(feature = "multibytedecode")]
            ("euc-kr".as_bytes(), b"\xbe\xc8\xb3\xe7\xc7\xcf\xbc\xbc\xbf\xe4 \xbc\xbc\xb0\xe8".to_vec(),"안녕하세요 세계"),
            #[cfg(feature = "multibytedecode")]
            ("iso-2022-jp".as_bytes(), b"\x1b$B%O%m!<!&%o!<%k%I\x1b(B".to_vec(),"ハロー・ワールド"),
            #[cfg(feature = "multibytedecode")]
            ("gbk".as_bytes(), b"\xc4\xe3\xba\xc3\xa3\xac\xca\xc0\xbd\xe7".to_vec(),"你好，世界"),
            #[cfg(feature = "multibytedecode")]
            ("gb18030".as_bytes(), b"\xc4\xe3\xba\xc3\xa3\xac\xca\xc0\xbd\xe7".to_vec(),"你好，世界"),
            #[cfg(feature = "multibytedecode")]
            ("utf-16".as_bytes(), b"\xff\xfe\xe1\x00\xe9\x00\xed\x00\xf3\x00\xfa\x00".to_vec(),"áéíóú"),
            ];

        for input in inputs {
            let mut decoder;

            for ch in input.0 {
                parser.ingest(*ch);
            }
            //parser.ingest_slice(input.0);
            decoder = parser.get_decoder(50).unwrap();
            decoder.ingest_slice(&input.1);

            let result = decoder.to_string().unwrap();
            //println!("{}", result);
            assert_eq!(result, input.2);

            parser.reset();
        }
    }
}

