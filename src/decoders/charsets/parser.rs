use super::{CharsetDecoder, single_byte::SingleByteDecoder};

#[cfg(feature = "multibytedecode")]
use super::multi_byte::MultiByteDecoder;

pub struct CharsetParser {
    charset: [u8; 45],
    len: u32,
    hash: u32
}

impl CharsetParser {
    pub fn new() -> CharsetParser {
        CharsetParser {
            charset: [0; 45],
            len: 0,
            hash: 0
        }
    }

    pub fn get_default_decoder(capacity: usize) -> Box<dyn CharsetDecoder> {
        Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))
    }

    pub fn ingest(&mut self, mut ch: u8) {
        if self.len < 45 {
            if (b'A'..=b'Z').contains(&ch) {
                ch += 32;
            }

            unsafe {
                *self.charset.get_unchecked_mut(self.len as usize) = ch;
                self.len += 1;

                match self.len {
                    2 | 9 | 11 => {
                        self.hash += *CH_HASH.get_unchecked((ch as usize) + 1);
                    },
                    8 => {
                        self.hash += *CH_HASH.get_unchecked((ch as usize) + 3);
                    },
                    1..=10 | 22 => {
                        self.hash += *CH_HASH.get_unchecked(ch as usize);
                    },
                    _ => ()
                }
            }
        }
    }

    pub fn ingest_slice(&mut self, chs: &[u8]) {
        for ch in chs {
            self.ingest(*ch);
        }
    }

    pub fn reset(&mut self) {
        self.hash = 0;
        self.len = 0;
    }

    #[inline(always)]
    pub fn get_charset(&mut self) -> &[u8] {
        &self.charset[0..self.len as usize]
    }

    pub fn get_decoder(&mut self, capacity: usize) -> Option<Box<dyn CharsetDecoder>> {
        if self.len < 2 {
            return None;
        }

        let hash = self.hash + 
                    self.len + 
                    unsafe { CH_HASH.get_unchecked(*self.charset
                                    .get_unchecked((self.len - 1) as usize) as usize) };

        if !(30..=5355).contains(&hash) {
            return None;
        }

        let charset = self.get_charset();

        // TODO: Find a way to avoid repeating code while retaining performance.
        // As of 2021, Rust does not support fallthrough or multiple guards per match arm.

        match hash - 30 {
            // ISO_8859-1:1987
            1399 if charset == b"iso-8859-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
            3227 if charset == b"iso_8859-1:1987" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
            140 if charset == b"iso-ir-100" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
            2114 if charset == b"iso_8859-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
            1596 if charset == b"latin1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
            622 if charset == b"l1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
            841 if charset == b"ibm819" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
            1065 if charset == b"cp819" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
            2891 if charset == b"csisolatin1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),        
            
            // ISO_8859-2:1987
            1389 if charset == b"iso-8859-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
            3222 if charset == b"iso_8859-2:1987" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
            40 if charset == b"iso-ir-101" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
            2104 if charset == b"iso_8859-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
            1586 if charset == b"latin2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
            1052 if charset == b"l2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
            3321 if charset == b"csisolatin2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
    
            // ISO_8859-3:1988
            2259 if charset == b"iso-8859-3" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_3, capacity))),
            3537 if charset == b"iso_8859-3:1988" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_3, capacity))),
            770 if charset == b"iso-ir-109" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_3, capacity))),
            2974 if charset == b"iso_8859-3" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_3, capacity))),
            2456 if charset == b"latin3" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_3, capacity))),
            1087 if charset == b"l3" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_3, capacity))),
            3356 if charset == b"csisolatin3" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_3, capacity))),
    
            // ISO_8859-4:1988
            1459 if charset == b"iso-8859-4" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_4, capacity))),
            3137 if charset == b"iso_8859-4:1988" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_4, capacity))),
            135 if charset == b"iso-ir-110" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_4, capacity))),
            2174 if charset == b"iso_8859-4" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_4, capacity))),
            1656 if charset == b"latin4" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_4, capacity))),
            672 if charset == b"l4" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_4, capacity))),
            2941 if charset == b"csisolatin4" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_4, capacity))),
    
            // ISO_8859-5:1988
            1429 if charset == b"iso-8859-5" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_5, capacity))),
            3122 if charset == b"iso_8859-5:1988" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_5, capacity))),
            115 if charset == b"iso-ir-144" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_5, capacity))),
            2144 if charset == b"iso_8859-5" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_5, capacity))),
            1483 if charset == b"cyrillic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_5, capacity))),
            3228 if charset == b"csisolatincyrillic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_5, capacity))),
    
            // ISO_8859-6:1987
            1679 if charset == b"iso-8859-6" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_6, capacity))),
            3367 if charset == b"iso_8859-6:1987" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_6, capacity))),
            840 if charset == b"iso-ir-127" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_6, capacity))),
            2394 if charset == b"iso_8859-6" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_6, capacity))),
            1448 if charset == b"ecma-114" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_6, capacity))),
            1926 if charset == b"asmo-708" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_6, capacity))),
            1766 if charset == b"arabic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_6, capacity))),
            3241 if charset == b"csisolatinarabic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_6, capacity))),
    
            // ISO_8859-7:1987
            1769 if charset == b"iso-8859-7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
            3412 if charset == b"iso_8859-7:1987" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
            750 if charset == b"iso-ir-126" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
            2484 if charset == b"iso_8859-7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
            2291 if charset == b"elot_928" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
            2216 if charset == b"ecma-118" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
            2952 if charset == b"greek" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
            2177 if charset == b"greek8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
            4612 if charset == b"csisolatingreek" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_7, capacity))),
    
            // ISO_8859-8:1988
            1529 if charset == b"iso-8859-8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_8, capacity))),
            3172 if charset == b"iso_8859-8:1988" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_8, capacity))),
            200 if charset == b"iso-ir-138" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_8, capacity))),
            2244 if charset == b"iso_8859-8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_8, capacity))),
            1692 if charset == b"hebrew" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_8, capacity))),
            3066 if charset == b"csisolatinhebrew" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_8, capacity))),
    
            // ISO_8859-9:1989
            2129 if charset == b"iso-8859-9" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_9, capacity))),
            3772 if charset == b"iso_8859-9:1989" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_9, capacity))),
            185 if charset == b"iso-ir-148" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_9, capacity))),
            2844 if charset == b"iso_8859-9" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_9, capacity))),
            1626 if charset == b"latin5" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_9, capacity))),
            782 if charset == b"l5" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_9, capacity))),
            3051 if charset == b"csisolatin5" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_9, capacity))),
    
            // ISO-8859-10
            1455 if charset == b"iso-8859-10" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_10, capacity))),
            550 if charset == b"iso-ir-157" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_10, capacity))),
            952 if charset == b"l6" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_10, capacity))),
            2120 if charset == b"iso_8859-10:1992" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_10, capacity))),
            3221 if charset == b"csisolatin6" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_10, capacity))),
            1876 if charset == b"latin6" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_10, capacity))),
    
            // ISO-8859-13
            1865 if charset == b"iso-8859-13" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_13, capacity))),
            844 if charset == b"csiso885913" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_13, capacity))),
    
            // ISO-8859-14
            1450 if charset == b"iso-8859-14" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_14, capacity))),
            853 if charset == b"iso-ir-199" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_14, capacity))),
            2205 if charset == b"iso_8859-14:1998" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_14, capacity))),
            2165 if charset == b"iso_8859-14" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_14, capacity))),
            1726 if charset == b"latin8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_14, capacity))),
            1735 if charset == b"iso-celtic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_14, capacity))),
            1057 if charset == b"l8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_14, capacity))),
            429 if charset == b"csiso885914" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_14, capacity))),
    
            // ISO-8859-15
            1560 if charset == b"iso-8859-15" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_15, capacity))),
            2275 if charset == b"iso_8859-15" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_15, capacity))),
            2327 if charset == b"latin-9" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_15, capacity))),
            539 if charset == b"csiso885915" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_15, capacity))),
    
            // ISO-8859-16
            1730 if charset == b"iso-8859-16" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_16, capacity))),
            735 if charset == b"iso-ir-226" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_16, capacity))),
            2310 if charset == b"iso_8859-16:2001" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_16, capacity))),
            2445 if charset == b"iso_8859-16" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_16, capacity))),
            1702 if charset == b"latin10" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_16, capacity))),
            728 if charset == b"l10" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_16, capacity))),
            709 if charset == b"csiso885916" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_16, capacity))),

            // IBM850
            226 if charset == b"ibm850" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::IBM850, capacity))),
            450 if charset == b"cp850" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::IBM850, capacity))),
            298 if charset == b"850" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::IBM850, capacity))),
            2899 if charset == b"cspc850multilingual" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::IBM850, capacity))),

            // KOI8-R
            1032 if charset == b"koi8-r" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::KOI8_R, capacity))),
            978 if charset == b"cskoi8r" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::KOI8_R, capacity))),
            
            // KOI8-U
            1822 if charset == b"koi8-u" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::KOI8_U, capacity))),
            1768 if charset == b"cskoi8u" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::KOI8_U, capacity))),

            // TIS-620
            1012 if charset == b"tis-620" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::TIS_620, capacity))),
            703 if charset == b"cstis620" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::TIS_620, capacity))),
            1400 if charset == b"iso-8859-11" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::TIS_620, capacity))),
            
            // WINDOWS-1250
            2302 if charset == b"windows-1250" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1250, capacity))),
            1763 if charset == b"cswindows1250" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1250, capacity))),
            
            // WINDOWS-1251
            2252 if charset == b"windows-1251" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1251, capacity))),
            1713 if charset == b"cswindows1251" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1251, capacity))),
            
            // WINDOWS-1252
            2247 if charset == b"windows-1252" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1252, capacity))),
            1708 if charset == b"cswindows1252" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1252, capacity))),
            
            // WINDOWS-1253
            2682 if charset == b"windows-1253" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1253, capacity))),
            2143 if charset == b"cswindows1253" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1253, capacity))),
            
            // WINDOWS-1254
            2282 if charset == b"windows-1254" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1254, capacity))),
            1743 if charset == b"cswindows1254" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1254, capacity))),
            
            // WINDOWS-1255
            2267 if charset == b"windows-1255" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1255, capacity))),
            1728 if charset == b"cswindows1255" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1255, capacity))),
            
            // WINDOWS-1256
            2392 if charset == b"windows-1256" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1256, capacity))),
            1853 if charset == b"cswindows1256" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1256, capacity))),
            
            // WINDOWS-1257
            2437 if charset == b"windows-1257" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1257, capacity))),
            1898 if charset == b"cswindows1257" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1257, capacity))),

            // WINDOWS-1258
            2317 if charset == b"windows-1258" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1258, capacity))),
            1778 if charset == b"cswindows1258" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::CP1258, capacity))),

            // MACINTOSH
            2730 if charset == b"macintosh" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::MACINTOSH, capacity))),
            368 if charset == b"mac" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::MACINTOSH, capacity))),
            2457 if charset == b"csmacintosh" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::MACINTOSH, capacity))),

            // SHIFT_JIS
            #[cfg(feature = "multibytedecode")]
            2234 if charset == b"shift_jis" => Some(Box::new(MultiByteDecoder::get_shift_jis(capacity))),
            #[cfg(feature = "multibytedecode")]
            4294 if charset == b"ms_kanji" => Some(Box::new(MultiByteDecoder::get_shift_jis(capacity))),
            #[cfg(feature = "multibytedecode")]
            1796 if charset == b"csshiftjis" => Some(Box::new(MultiByteDecoder::get_shift_jis(capacity))),

            // BIG5
            #[cfg(feature = "multibytedecode")]
            1729 if charset == b"big5" => Some(Box::new(MultiByteDecoder::get_big5(capacity))),
            #[cfg(feature = "multibytedecode")]
            1041 if charset == b"csbig5" => Some(Box::new(MultiByteDecoder::get_big5(capacity))),

            // EXTENDED_UNIX_CODE_PACKED_FORMAT_FOR_JAPANESE
            #[cfg(feature = "multibytedecode")]
            1951 if charset == b"euc-jp" => Some(Box::new(MultiByteDecoder::get_euc_jp(capacity))),
            #[cfg(feature = "multibytedecode")]
            2835 if charset == b"extended_unix_code_packed_format_for_japanese" => Some(Box::new(MultiByteDecoder::get_euc_jp(capacity))),
            #[cfg(feature = "multibytedecode")]
            3555 if charset == b"cseucpkdfmtjapanese" => Some(Box::new(MultiByteDecoder::get_euc_jp(capacity))),

            // EUC-KR
            #[cfg(feature = "multibytedecode")]
            2032 if charset == b"euc-kr" => Some(Box::new(MultiByteDecoder::get_euc_kr(capacity))),
            #[cfg(feature = "multibytedecode")]
            1508 if charset == b"cseuckr" => Some(Box::new(MultiByteDecoder::get_euc_kr(capacity))),

            // ISO-2022-JP
            #[cfg(feature = "multibytedecode")]
            2280 if charset == b"iso-2022-jp" => Some(Box::new(MultiByteDecoder::get_iso2022_jp(capacity))),
            #[cfg(feature = "multibytedecode")]
            1616 if charset == b"csiso2022jp" => Some(Box::new(MultiByteDecoder::get_iso2022_jp(capacity))),

            // GB18030
            #[cfg(feature = "multibytedecode")]
            1332 if charset == b"gb18030" => Some(Box::new(MultiByteDecoder::get_gb18030(capacity))),
            #[cfg(feature = "multibytedecode")]
            1334 if charset == b"csgb18030" => Some(Box::new(MultiByteDecoder::get_gb18030(capacity))),

            // GBK
            #[cfg(feature = "multibytedecode")]
            2485 if charset == b"gbk" => Some(Box::new(MultiByteDecoder::get_gbk(capacity))),
            #[cfg(feature = "multibytedecode")]
            1345 if charset == b"cp936" => Some(Box::new(MultiByteDecoder::get_gbk(capacity))),
            #[cfg(feature = "multibytedecode")]
            1105 if charset == b"ms936" => Some(Box::new(MultiByteDecoder::get_gbk(capacity))),
            #[cfg(feature = "multibytedecode")]
            2959 if charset == b"windows-936" => Some(Box::new(MultiByteDecoder::get_gbk(capacity))),
            #[cfg(feature = "multibytedecode")]
            2827 if charset == b"csgbk" => Some(Box::new(MultiByteDecoder::get_gbk(capacity))),

            // UTF-16BE
            #[cfg(feature = "multibytedecode")]
            2294 if charset == b"utf-16be" => Some(Box::new(MultiByteDecoder::get_utf16_be(capacity))),
            #[cfg(feature = "multibytedecode")]
            994 if charset == b"csutf16be" => Some(Box::new(MultiByteDecoder::get_utf16_be(capacity))),

            // UTF-16LE
            #[cfg(feature = "multibytedecode")]
            2599 if charset == b"utf-16le" => Some(Box::new(MultiByteDecoder::get_utf16_le(capacity))),
            #[cfg(feature = "multibytedecode")]
            794 if charset == b"csutf16le" => Some(Box::new(MultiByteDecoder::get_utf16_le(capacity))),

            // UTF-16
            #[cfg(feature = "multibytedecode")]
            1091 if charset == b"utf-16" => Some(Box::new(MultiByteDecoder::get_utf16_le(capacity))),
            #[cfg(feature = "multibytedecode")]
            707 if charset == b"csutf16" => Some(Box::new(MultiByteDecoder::get_utf16_le(capacity))),
            
            // UTF-8, US-ASCII and other encodings fall back here
            _ => None
        }
    }
}

// Perfect hashing table for charset names
static CH_HASH: &[u32] = &[
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 0, 5356, 5356, 5356, 0, 1184, 20, 55, 5,
    0, 435, 35, 20, 145, 190, 70, 370, 88, 923, 60, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 715, 0, 675, 340, 10, 325, 200, 30,
    670, 806, 5, 705, 916, 645, 35, 620, 0, 65, 265, 0, 75, 0, 395, 930, 170, 711, 0, 155, 0, 0,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356, 5356,
    5356, 5356, 5356, 5356, 5356, 5356
];

#[cfg(test)]
mod tests {
    use super::CharsetParser;

    #[test]
    fn find_decoder() {
        let inputs = [
            "ISO-8859-1".as_bytes(),
            "csisolatingreek".as_bytes(),
            "csisolatincyrillic".as_bytes(),
            "Cspc850MultiLingual".as_bytes(),
            #[cfg(feature = "multibytedecode")]
            "extended_unix_code_packed_format_for_japanese".as_bytes()
            ];
        let mut parser = CharsetParser::new();

        for input in inputs {
            parser.ingest_slice(input);
            match parser.get_decoder(1) {
                Some(_) => (),
                None => {
                    panic!("Could not find a decoder for '{}'.", std::str::from_utf8(input).unwrap());
                }
            }
            parser.reset();
        }
    }
}

/*
    
    Generated from http://www.iana.org/assignments/character-sets/character-sets.xhtml
    Keep for future support of additional character sets.

        // UTF-8
        935 if charset == b"utf-8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        551 if charset == b"csutf8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),

        // US-ASCII
        1788 if charset == b"us-ascii" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        503 if charset == b"iso-ir-6" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        2695 if charset == b"ansi_x3.4-1968" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        2770 if charset == b"ansi_x3.4-1986" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        1916 if charset == b"iso_646.irv:1991" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        1095 if charset == b"iso646-us" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        442 if charset == b"us" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        986 if charset == b"ibm367" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        1210 if charset == b"cp367" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),
        762 if charset == b"csascii" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_1, capacity))),

        // ISO_6937-2-ADD
        4435 if charset == b"iso_6937-2-add" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        45 if charset == b"iso-ir-142" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        778 if charset == b"csisotextcomm" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_X0201
        3385 if charset == b"jis_x0201" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        756 if charset == b"x0201" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3850 if charset == b"cshalfwidthkatakana" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_ENCODING
        3887 if charset == b"jis_encoding" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3393 if charset == b"csjisencoding" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EXTENDED_UNIX_CODE_FIXED_WIDTH_FOR_JAPANESE
        3534 if charset == b"extended_unix_code_fixed_width_for_japanese" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1755 if charset == b"cseucfixwidjapanese" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // BS_4730
        1802 if charset == b"bs_4730" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        213 if charset == b"iso-ir-4" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1364 if charset == b"iso646-gb" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        992 if charset == b"gb" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1928 if charset == b"uk" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1544 if charset == b"csiso4unitedkingdom" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // SEN_850200_C
        1657 if charset == b"sen_850200_c" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        29 if charset == b"iso-ir-11" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1270 if charset == b"iso646-se2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        78 if charset == b"se2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2283 if charset == b"csiso11swedishfornames" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IT
        372 if charset == b"it" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        189 if charset == b"iso-ir-15" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1349 if charset == b"iso646-it" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2469 if charset == b"csiso15italian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ES
        247 if charset == b"es" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        284 if charset == b"iso-ir-17" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1190 if charset == b"iso646-es" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2945 if charset == b"csiso17spanish" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // DIN_66003
        3594 if charset == b"din_66003" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        14 if charset == b"iso-ir-21" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        527 if charset == b"de" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1209 if charset == b"iso646-de" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2053 if charset == b"csiso21german" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NS_4551-1
        1454 if charset == b"ns_4551-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        419 if charset == b"iso-ir-60" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        639 if charset == b"iso646-no" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        657 if charset == b"no" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3857 if charset == b"csiso60danishnorwegian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1323 if charset == b"csiso60norwegian1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NF_Z_62-010
        3126 if charset == b"nf_z_62-010" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        817 if charset == b"iso-ir-69" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        389 if charset == b"iso646-fr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        77 if charset == b"fr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1674 if charset == b"csiso69french" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-10646-UTF-1
        1510 if charset == b"iso-10646-utf-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1604 if charset == b"csiso10646utf1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_646.BASIC:1983
        2158 if charset == b"iso_646.basic:1983" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        63 if charset == b"ref" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1747 if charset == b"csiso646basic1983" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // INVARIANT
        2929 if charset == b"invariant" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        4221 if charset == b"csinvariant" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_646.IRV:1983
        2346 if charset == b"iso_646.irv:1983" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        8 if charset == b"iso-ir-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1913 if charset == b"irv" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2085 if charset == b"csiso2intlrefversion" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NATS-SEFI
        2004 if charset == b"nats-sefi" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2107 if charset == b"iso-ir-8-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2921 if charset == b"csnatssefi" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NATS-SEFI-ADD
        2668 if charset == b"nats-sefi-add" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2097 if charset == b"iso-ir-8-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3584 if charset == b"csnatssefiadd" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NATS-DANO
        2344 if charset == b"nats-dano" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1244 if charset == b"iso-ir-9-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2010 if charset == b"csnatsdano" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NATS-DANO-ADD
        3013 if charset == b"nats-dano-add" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1234 if charset == b"iso-ir-9-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2678 if charset == b"csnatsdanoadd" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // SEN_850200_B
        1987 if charset == b"sen_850200_b" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        84 if charset == b"iso-ir-10" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        712 if charset == b"fi" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1024 if charset == b"iso646-fi" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1469 if charset == b"iso646-se" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        277 if charset == b"se" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2981 if charset == b"csiso10swedish" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // KS_C_5601-1987
        3130 if charset == b"ks_c_5601-1987" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        785 if charset == b"iso-ir-149" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3310 if charset == b"ks_c_5601-1989" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1879 if charset == b"ksc_5601" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3072 if charset == b"korean" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1877 if charset == b"csksc56011987" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-2022-KR
        2236 if charset == b"iso-2022-kr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1572 if charset == b"csiso2022kr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-2022-JP-2
        2217 if charset == b"iso-2022-jp-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1552 if charset == b"csiso2022jp2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6220-1969-JP
        2432 if charset == b"jis_c6220-1969-jp" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2734 if charset == b"jis_c6220-1969" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        494 if charset == b"iso-ir-13" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        5120 if charset == b"katakana" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1133 if charset == b"x0201-7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1722 if charset == b"csiso13jisc6220jp" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6220-1969-RO
        2367 if charset == b"jis_c6220-1969-ro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        79 if charset == b"iso-ir-14" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1007 if charset == b"jp" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        674 if charset == b"iso646-jp" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1257 if charset == b"csiso14jisc6220ro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // PT
        432 if charset == b"pt" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        359 if charset == b"iso-ir-16" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        779 if charset == b"iso646-pt" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        962 if charset == b"csiso16portuguese" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // GREEK7-OLD
        2916 if charset == b"greek7-old" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        464 if charset == b"iso-ir-18" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1486 if charset == b"csiso18greek7old" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // LATIN-GREEK
        4447 if charset == b"latin-greek" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        482 if charset == b"iso-ir-19" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2413 if charset == b"csiso19latingreek" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NF_Z_62-010_(1973)
        3078 if charset == b"nf_z_62-010_(1973)" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        174 if charset == b"iso-ir-25" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        400 if charset == b"iso646-fr1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1179 if charset == b"csiso25french" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // LATIN-GREEK-1
        3538 if charset == b"latin-greek-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        269 if charset == b"iso-ir-27" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1318 if charset == b"csiso27latingreek1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_5427
        1031 if charset == b"iso_5427" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        394 if charset == b"iso-ir-37" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        397 if charset == b"csiso5427cyrillic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6226-1978
        2619 if charset == b"jis_c6226-1978" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        614 if charset == b"iso-ir-42" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1324 if charset == b"csiso42jisc62261978" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // BS_VIEWDATA
        4366 if charset == b"bs_viewdata" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        439 if charset == b"iso-ir-47" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2812 if charset == b"csiso47bsviewdata" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // INIS
        134 if charset == b"inis" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        637 if charset == b"iso-ir-49" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1201 if charset == b"csiso49inis" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // INIS-8
        201 if charset == b"inis-8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        119 if charset == b"iso-ir-50" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        867 if charset == b"csiso50inis8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // INIS-CYRILLIC
        1868 if charset == b"inis-cyrillic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        64 if charset == b"iso-ir-51" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        764 if charset == b"csiso51iniscyrillic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_5427:1981
        1867 if charset == b"iso_5427:1981" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        114 if charset == b"iso-ir-54" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1134 if charset == b"iso5427cyrillic1981" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        316 if charset == b"csiso54271981" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_5428:1980
        2752 if charset == b"iso_5428:1980" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        224 if charset == b"iso-ir-55" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2180 if charset == b"csiso5428greek" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // GB_1988-80
        2425 if charset == b"gb_1988-80" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        319 if charset == b"iso-ir-57" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        602 if charset == b"cn" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        959 if charset == b"iso646-cn" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1161 if charset == b"csiso57gb1988" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // GB_2312-80
        2350 if charset == b"gb_2312-80" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        499 if charset == b"iso-ir-58" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1292 if charset == b"chinese" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        970 if charset == b"csiso58gb231280" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NS_4551-2
        1884 if charset == b"ns_4551-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        640 if charset == b"iso646-no2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        364 if charset == b"iso-ir-61" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        658 if charset == b"no2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1268 if charset == b"csiso61norwegian2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // VIDEOTEX-SUPPL
        5178 if charset == b"videotex-suppl" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        137 if charset == b"iso-ir-70" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1390 if charset == b"csiso70videotexsupp1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // PT2
        433 if charset == b"pt2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        967 if charset == b"iso-ir-84" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        780 if charset == b"iso646-pt2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        718 if charset == b"csiso84portuguese2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ES2
        173 if charset == b"es2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1077 if charset == b"iso-ir-85" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1116 if charset == b"iso646-es2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2035 if charset == b"csiso85spanish2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // MSZ_7795.3
        2595 if charset == b"msz_7795.3" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1247 if charset == b"iso-ir-86" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2550 if charset == b"iso646-hu" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2103 if charset == b"hu" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        4183 if charset == b"csiso86hungarian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6226-1983
        2984 if charset == b"jis_c6226-1983" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1172 if charset == b"iso-ir-87" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        886 if charset == b"x0208" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        4190 if charset == b"jis_x0208-1983" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1220 if charset == b"csiso87jisx0208" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // GREEK7
        2417 if charset == b"greek7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1352 if charset == b"iso-ir-88" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1413 if charset == b"csiso88greek7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ASMO_449
        1903 if charset == b"asmo_449" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2073 if charset == b"iso_9036" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2137 if charset == b"arabic7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1370 if charset == b"iso-ir-89" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1309 if charset == b"csiso89asmo449" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-IR-90
        109 if charset == b"iso-ir-90" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        547 if charset == b"csiso90" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6229-1984-A
        3124 if charset == b"jis_c6229-1984-a" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        54 if charset == b"iso-ir-91" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1958 if charset == b"jp-ocr-a" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2270 if charset == b"csiso91jisc62291984a" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6229-1984-B
        2789 if charset == b"jis_c6229-1984-b" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        484 if charset == b"iso-ir-92" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1020 if charset == b"iso646-jp-ocr-b" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1498 if charset == b"jp-ocr-b" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1930 if charset == b"csiso92jisc62991984b" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6229-1984-B-ADD
        2778 if charset == b"jis_c6229-1984-b-add" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        519 if charset == b"iso-ir-93" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3546 if charset == b"jp-ocr-b-add" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2542 if charset == b"csiso93jis62291984badd" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6229-1984-HAND
        2777 if charset == b"jis_c6229-1984-hand" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        104 if charset == b"iso-ir-94" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3362 if charset == b"jp-ocr-hand" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2142 if charset == b"csiso94jis62291984hand" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6229-1984-HAND-ADD
        3106 if charset == b"jis_c6229-1984-hand-add" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        214 if charset == b"iso-ir-95" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3366 if charset == b"jp-ocr-hand-add" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2130 if charset == b"csiso95jis62291984handadd" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_C6229-1984-KANA
        3127 if charset == b"jis_c6229-1984-kana" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        384 if charset == b"iso-ir-96" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3033 if charset == b"csiso96jisc62291984kana" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_2033-1983
        3050 if charset == b"iso_2033-1983" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        489 if charset == b"iso-ir-98" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1289 if charset == b"e13b" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        739 if charset == b"csiso2033" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ANSI_X3.110-1983
        3052 if charset == b"ansi_x3.110-1983" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        507 if charset == b"iso-ir-99" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3605 if charset == b"csa_t500-1983" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1861 if charset == b"naplps" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1593 if charset == b"csiso99naplps" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // T.61-7BIT
        1719 if charset == b"t.61-7bit" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        30 if charset == b"iso-ir-102" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        695 if charset == b"csiso102t617bit" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // T.61-8BIT
        1599 if charset == b"t.61-8bit" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        149 if charset == b"t.61" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        900 if charset == b"iso-ir-103" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        820 if charset == b"csiso103t618bit" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ECMA-CYRILLIC
        3018 if charset == b"ecma-cyrillic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        35 if charset == b"iso-ir-111" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1432 if charset == b"koi8-e" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        795 if charset == b"csiso111ecmacyrillic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // CSA_Z243.4-1985-1
        2966 if charset == b"csa_z243.4-1985-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        470 if charset == b"iso-ir-121" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1354 if charset == b"iso646-ca" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        861 if charset == b"csa7-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        860 if charset == b"csa71" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        997 if charset == b"ca" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1122 if charset == b"csiso121canadian1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // CSA_Z243.4-1985-2
        2961 if charset == b"csa_z243.4-1985-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        460 if charset == b"iso-ir-122" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        680 if charset == b"iso646-ca2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        851 if charset == b"csa7-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        850 if charset == b"csa72" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1102 if charset == b"csiso122canadian2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // CSA_Z243.4-1985-GR
        2962 if charset == b"csa_z243.4-1985-gr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1330 if charset == b"iso-ir-123" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        972 if charset == b"csiso123csaz24341985gr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_8859-6-E
        2920 if charset == b"iso-8859-6-e" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3635 if charset == b"iso_8859-6-e" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        744 if charset == b"csiso88596e" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_8859-6-I
        2725 if charset == b"iso-8859-6-i" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3440 if charset == b"iso_8859-6-i" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1224 if charset == b"csiso88596i" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // T.101-G2
        753 if charset == b"t.101-g2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        600 if charset == b"iso-ir-128" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1407 if charset == b"csiso128t101g2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_8859-8-E
        2845 if charset == b"iso-8859-8-e" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3560 if charset == b"iso_8859-8-e" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        669 if charset == b"csiso88598e" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_8859-8-I
        2650 if charset == b"iso-8859-8-i" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3365 if charset == b"iso_8859-8-i" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1149 if charset == b"csiso88598i" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // CSN_369103
        3185 if charset == b"csn_369103" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        800 if charset == b"iso-ir-139" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1412 if charset == b"csiso139csn369103" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JUS_I.B1.002
        4051 if charset == b"jus_i.b1.002" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        55 if charset == b"iso-ir-141" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1634 if charset == b"iso646-yu" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        752 if charset == b"js" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1297 if charset == b"yu" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1463 if charset == b"csiso141jusib1002" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IEC_P27-1
        1054 if charset == b"iec_p27-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        915 if charset == b"iso-ir-143" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1495 if charset == b"csiso143iecp271" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JUS_I.B1.003-SERB
        4396 if charset == b"jus_i.b1.003-serb" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        335 if charset == b"iso-ir-146" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2342 if charset == b"serbian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1380 if charset == b"csiso146serbian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JUS_I.B1.003-MAC
        4065 if charset == b"jus_i.b1.003-mac" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3735 if charset == b"macedonian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        425 if charset == b"iso-ir-147" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2446 if charset == b"csiso147macedonian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // GREEK-CCITT
        3182 if charset == b"greek-ccitt" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        280 if charset == b"iso-ir-150" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        583 if charset == b"csiso150" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1374 if charset == b"csiso150greekccitt" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // NC_NC00-10:81
        3421 if charset == b"nc_nc00-10:81" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2604 if charset == b"cuba" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        180 if charset == b"iso-ir-151" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1664 if charset == b"iso646-cu" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1537 if charset == b"csiso151cuba" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_6937-2-25
        4129 if charset == b"iso_6937-2-25" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        170 if charset == b"iso-ir-152" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2002 if charset == b"csiso6937add" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // GOST_19768-74
        3450 if charset == b"gost_19768-74" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        4702 if charset == b"st_sev_358-88" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1040 if charset == b"iso-ir-153" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1090 if charset == b"csiso153gost1976874" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_8859-SUPP
        3177 if charset == b"iso_8859-supp" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        240 if charset == b"iso-ir-154" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2839 if charset == b"latin1-2-5" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1441 if charset == b"csiso8859supp" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO_10367-BOX
        2359 if charset == b"iso_10367-box" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        210 if charset == b"iso-ir-155" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1379 if charset == b"csiso10367box" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // LATIN-LAP
        2889 if charset == b"latin-lap" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1088 if charset == b"lap" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        310 if charset == b"iso-ir-158" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2059 if charset == b"csiso158lap" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // JIS_X0212-1990
        3475 if charset == b"jis_x0212-1990" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        696 if charset == b"x0212" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        910 if charset == b"iso-ir-159" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1141 if charset == b"csiso159jisx02121990" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // DS_2089
        1882 if charset == b"ds_2089" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1166 if charset == b"ds2089" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2540 if charset == b"iso646-dk" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1858 if charset == b"dk" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2305 if charset == b"csiso646danish" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // US-DK
        2527 if charset == b"us-dk" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2613 if charset == b"csusdk" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // DK-US
        1490 if charset == b"dk-us" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1772 if charset == b"csdkus" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // KSC5636
        1793 if charset == b"ksc5636" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1004 if charset == b"iso646-kr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1635 if charset == b"csksc5636" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // UNICODE-1-1-UTF-7
        1167 if charset == b"unicode-1-1-utf-7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1920 if charset == b"csunicode11utf7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-2022-CN
        1875 if charset == b"iso-2022-cn" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1211 if charset == b"csiso2022cn" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-2022-CN-EXT
        1259 if charset == b"iso-2022-cn-ext" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        594 if charset == b"csiso2022cnext" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),


        // OSD_EBCDIC_DF04_15
        2983 if charset == b"osd_ebcdic_df04_15" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1517 if charset == b"csosdebcdicdf0415" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // OSD_EBCDIC_DF03_IRV
        3894 if charset == b"osd_ebcdic_df03_irv" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2428 if charset == b"csosdebcdicdf03irv" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // OSD_EBCDIC_DF04_1
        2967 if charset == b"osd_ebcdic_df04_1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1501 if charset == b"csosdebcdicdf041" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-11548-1
        581 if charset == b"iso-11548-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1296 if charset == b"iso_11548-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1499 if charset == b"iso_tr_11548-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        246 if charset == b"csiso115481" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // KZ-1048
        1128 if charset == b"kz-1048" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3576 if charset == b"strk1048-2002" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        856 if charset == b"rk1048" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2147 if charset == b"cskz1048" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-10646-UCS-2
        1505 if charset == b"iso-10646-ucs-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1919 if charset == b"csunicode" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-10646-UCS-4
        1540 if charset == b"iso-10646-ucs-4" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        536 if charset == b"csucs4" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-10646-UCS-BASIC
        1519 if charset == b"iso-10646-ucs-basic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2404 if charset == b"csunicodeascii" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-10646-UNICODE-LATIN1
        1524 if charset == b"iso-10646-unicode-latin1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2715 if charset == b"csunicodelatin1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        714 if charset == b"iso-10646" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-10646-J-1
        1494 if charset == b"iso-10646-j-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2972 if charset == b"csunicodejapanese" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-UNICODE-IBM-1261
        1470 if charset == b"iso-unicode-ibm-1261" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1746 if charset == b"csunicodeibm1261" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-UNICODE-IBM-1268
        1535 if charset == b"iso-unicode-ibm-1268" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1811 if charset == b"csunicodeibm1268" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-UNICODE-IBM-1276
        1610 if charset == b"iso-unicode-ibm-1276" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1886 if charset == b"csunicodeibm1276" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-UNICODE-IBM-1264
        1500 if charset == b"iso-unicode-ibm-1264" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1776 if charset == b"csunicodeibm1264" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-UNICODE-IBM-1265
        1485 if charset == b"iso-unicode-ibm-1265" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1761 if charset == b"csunicodeibm1265" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // UNICODE-1-1
        976 if charset == b"unicode-1-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1731 if charset == b"csunicode11" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // SCSU
        1239 if charset == b"scsu" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        936 if charset == b"csscsu" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // UTF-7
        1175 if charset == b"utf-7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        791 if charset == b"csutf7" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // CESU-8
        626 if charset == b"cesu-8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        807 if charset == b"cscesu8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1661 if charset == b"cscesu-8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // UTF-32
        1231 if charset == b"utf-32" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        847 if charset == b"csutf32" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // UTF-32BE
        2579 if charset == b"utf-32be" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1279 if charset == b"csutf32be" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // UTF-32LE
        2884 if charset == b"utf-32le" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1079 if charset == b"csutf32le" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // BOCU-1
        796 if charset == b"bocu-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        742 if charset == b"csbocu1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        773 if charset == b"csbocu-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // UTF-7-IMAP
        1530 if charset == b"utf-7-imap" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1145 if charset == b"csutf7imap" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-8859-1-WINDOWS-3.0-LATIN-1
        2658 if charset == b"iso-8859-1-windows-3.0-latin-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1717 if charset == b"cswindows30latin1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-8859-1-WINDOWS-3.1-LATIN-1
        2608 if charset == b"iso-8859-1-windows-3.1-latin-1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1712 if charset == b"cswindows31latin1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-8859-2-WINDOWS-LATIN-2
        2589 if charset == b"iso-8859-2-windows-latin-2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1707 if charset == b"cswindows31latin2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ISO-8859-9-WINDOWS-LATIN-5
        2979 if charset == b"iso-8859-9-windows-latin-5" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1727 if charset == b"cswindows31latin5" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // HP-ROMAN8
        2465 if charset == b"hp-roman8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1511 if charset == b"roman8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        412 if charset == b"r8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1361 if charset == b"cshproman8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ADOBE-STANDARD-ENCODING
        4103 if charset == b"adobe-standard-encoding" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        4833 if charset == b"csadobestandardencoding" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // VENTURA-US
        3765 if charset == b"ventura-us" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3341 if charset == b"csventuraus" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // VENTURA-INTERNATIONAL
        5061 if charset == b"ventura-international" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        4177 if charset == b"csventurainternational" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // DEC-MCS
        537 if charset == b"dec-mcs" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        348 if charset == b"dec" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1573 if charset == b"csdecmcs" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),

        // PC8-DANISH-NORWEGIAN
        5325 if charset == b"pc8-danish-norwegian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2815 if charset == b"cspc8danishnorwegian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM862
        241 if charset == b"ibm862" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        465 if charset == b"cp862" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        233 if charset == b"862" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1503 if charset == b"cspc862latinhebrew" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // PC8-TURKISH
        3047 if charset == b"pc8-turkish" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2383 if charset == b"cspc8turkish" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM-SYMBOLS
        1126 if charset == b"ibm-symbols" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        632 if charset == b"csibmsymbols" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM-THAI
        2159 if charset == b"ibm-thai" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2210 if charset == b"csibmthai" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // HP-LEGAL
        3884 if charset == b"hp-legal" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3380 if charset == b"cshplegal" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // HP-PI-FONT
        1151 if charset == b"hp-pi-font" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        961 if charset == b"cshppifont" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // HP-MATH8
        3558 if charset == b"hp-math8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2926 if charset == b"cshpmath8" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // ADOBE-SYMBOL-ENCODING
        3176 if charset == b"adobe-symbol-encoding" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3368 if charset == b"cshppsmath" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // HP-DESKTOP
        2932 if charset == b"hp-desktop" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2807 if charset == b"cshpdesktop" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // VENTURA-MATH
        5183 if charset == b"ventura-math" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        4054 if charset == b"csventuramath" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // MICROSOFT-PUBLISHING
        2150 if charset == b"microsoft-publishing" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2141 if charset == b"csmicrosoftpublishing" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // WINDOWS-31J
        3762 if charset == b"windows-31j" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2407 if charset == b"cswindows31j" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // GB2312
        1096 if charset == b"gb2312" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1458 if charset == b"csgb2312" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM037
        896 if charset == b"ibm037" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1120 if charset == b"cp037" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1842 if charset == b"ebcdic-cp-us" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1837 if charset == b"ebcdic-cp-ca" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1548 if charset == b"ebcdic-cp-wt" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1482 if charset == b"ebcdic-cp-nl" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1136 if charset == b"csibm037" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM038
        656 if charset == b"ibm038" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1185 if charset == b"ebcdic-int" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        880 if charset == b"cp038" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1851 if charset == b"csibm038" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM273
        1086 if charset == b"ibm273" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1310 if charset == b"cp273" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1138 if charset == b"csibm273" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM274
        286 if charset == b"ibm274" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        969 if charset == b"ebcdic-be" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        510 if charset == b"cp274" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        783 if charset == b"csibm274" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM275
        256 if charset == b"ibm275" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        814 if charset == b"ebcdic-br" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        480 if charset == b"cp275" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        648 if charset == b"csibm275" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM277
        596 if charset == b"ibm277" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1953 if charset == b"ebcdic-cp-dk" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        837 if charset == b"ebcdic-cp-no" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        836 if charset == b"csibm277" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM278
        356 if charset == b"ibm278" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        580 if charset == b"cp278" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1512 if charset == b"ebcdic-cp-fi" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1037 if charset == b"ebcdic-cp-se" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1551 if charset == b"csibm278" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM280
        206 if charset == b"ibm280" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        430 if charset == b"cp280" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1542 if charset == b"ebcdic-cp-it" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        928 if charset == b"csibm280" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM281
        106 if charset == b"ibm281" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1071 if charset == b"ebcdic-jp-e" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        330 if charset == b"cp281" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        478 if charset == b"csibm281" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM284
        166 if charset == b"ibm284" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        390 if charset == b"cp284" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        942 if charset == b"ebcdic-cp-es" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        663 if charset == b"csibm284" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM285
        136 if charset == b"ibm285" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        360 if charset == b"cp285" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1983 if charset == b"ebcdic-cp-gb" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        528 if charset == b"csibm285" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM290
        506 if charset == b"ibm290" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        730 if charset == b"cp290" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2164 if charset == b"ebcdic-jp-kana" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1228 if charset == b"csibm290" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM297
        776 if charset == b"ibm297" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1000 if charset == b"cp297" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1507 if charset == b"ebcdic-cp-fr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1016 if charset == b"csibm297" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM420
        171 if charset == b"ibm420" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        395 if charset == b"cp420" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1183 if charset == b"ebcdic-cp-ar1" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        893 if charset == b"csibm420" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM423
        931 if charset == b"ibm423" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1155 if charset == b"cp423" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1643 if charset == b"ebcdic-cp-gr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        983 if charset == b"csibm423" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM424
        131 if charset == b"ibm424" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        355 if charset == b"cp424" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1042 if charset == b"ebcdic-cp-he" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        628 if charset == b"csibm424" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM437
        876 if charset == b"ibm437" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1100 if charset == b"cp437" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        423 if charset == b"437" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1446 if charset == b"cspc8codepage437" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM500
        211 if charset == b"ibm500" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        435 if charset == b"cp500" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1047 if charset == b"ebcdic-cp-be" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1968 if charset == b"ebcdic-cp-ch" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        933 if charset == b"csibm500" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM851
        126 if charset == b"ibm851" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        350 if charset == b"cp851" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        198 if charset == b"851" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        498 if charset == b"csibm851" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM852
        116 if charset == b"ibm852" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        340 if charset == b"cp852" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        188 if charset == b"852" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        238 if charset == b"cspcp852" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM855
        156 if charset == b"ibm855" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        380 if charset == b"cp855" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        228 if charset == b"855" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        548 if charset == b"csibm855" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM857
        496 if charset == b"ibm857" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        720 if charset == b"cp857" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        568 if charset == b"857" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        736 if charset == b"csibm857" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM860
        351 if charset == b"ibm860" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        575 if charset == b"cp860" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        343 if charset == b"860" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1073 if charset == b"csibm860" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM861
        251 if charset == b"ibm861" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        475 if charset == b"cp861" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        243 if charset == b"861" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        405 if charset == b"cp-is" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        623 if charset == b"csibm861" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM863
        1111 if charset == b"ibm863" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1335 if charset == b"cp863" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1103 if charset == b"863" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1163 if charset == b"csibm863" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM864
        311 if charset == b"ibm864" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        535 if charset == b"cp864" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        808 if charset == b"csibm864" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM865
        281 if charset == b"ibm865" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        505 if charset == b"cp865" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        273 if charset == b"865" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        673 if charset == b"csibm865" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM868
        381 if charset == b"ibm868" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        605 if charset == b"cp868" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        925 if charset == b"cp-ar" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1576 if charset == b"csibm868" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM869
        981 if charset == b"ibm869" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1205 if charset == b"cp869" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        973 if charset == b"869" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        920 if charset == b"cp-gr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1013 if charset == b"csibm869" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM870
        396 if charset == b"ibm870" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        620 if charset == b"cp870" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1115 if charset == b"ebcdic-cp-roece" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1387 if charset == b"ebcdic-cp-yu" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1118 if charset == b"csibm870" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM871
        296 if charset == b"ibm871" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        520 if charset == b"cp871" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1617 if charset == b"ebcdic-cp-is" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        668 if charset == b"csibm871" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM880
        276 if charset == b"ibm880" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        500 if charset == b"cp880" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1445 if charset == b"ebcdic-cyrillic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        998 if charset == b"csibm880" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM891
        476 if charset == b"ibm891" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        700 if charset == b"cp891" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        848 if charset == b"csibm891" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM903
        1321 if charset == b"ibm903" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1545 if charset == b"cp903" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1373 if charset == b"csibm903" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM904
        521 if charset == b"ibm904" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        745 if charset == b"cp904" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        418 if charset == b"904" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1569 if charset == b"csibbm904" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM905
        491 if charset == b"ibm905" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        715 if charset == b"cp905" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1232 if charset == b"ebcdic-cp-tr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        883 if charset == b"csibm905" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM918
        541 if charset == b"ibm918" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        765 if charset == b"cp918" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1178 if charset == b"ebcdic-cp-ar2" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1736 if charset == b"csibm918" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM1026
        377 if charset == b"ibm1026" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        601 if charset == b"cp1026" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        784 if charset == b"csibm1026" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-AT-DE
        1662 if charset == b"ebcdic-at-de" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1675 if charset == b"csibmebcdicatde" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-AT-DE-A
        2139 if charset == b"ebcdic-at-de-a" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2118 if charset == b"csebcdicatdea" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-CA-FR
        1582 if charset == b"ebcdic-ca-fr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2572 if charset == b"csebcdiccafr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-DK-NO
        1857 if charset == b"ebcdic-dk-no" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2018 if charset == b"csebcdicdkno" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-DK-NO-A
        2534 if charset == b"ebcdic-dk-no-a" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2694 if charset == b"csebcdicdknoa" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-FI-SE
        1452 if charset == b"ebcdic-fi-se" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1777 if charset == b"csebcdicfise" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-FI-SE-A
        1929 if charset == b"ebcdic-fi-se-a" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2253 if charset == b"csebcdicfisea" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-FR
        619 if charset == b"ebcdic-fr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1570 if charset == b"csebcdicfr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-IT
        1579 if charset == b"ebcdic-it" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1605 if charset == b"csebcdicit" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-PT
        1009 if charset == b"ebcdic-pt" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1165 if charset == b"csebcdicpt" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-ES
        1420 if charset == b"ebcdic-es" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1080 if charset == b"csebcdices" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-ES-A
        2362 if charset == b"ebcdic-es-a" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2021 if charset == b"csebcdicesa" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-ES-S
        1422 if charset == b"ebcdic-es-s" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1081 if charset == b"csebcdicess" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-UK
        2811 if charset == b"ebcdic-uk" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3662 if charset == b"csebcdicuk" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // EBCDIC-US
        1325 if charset == b"ebcdic-us" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1980 if charset == b"csebcdicus" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // UNKNOWN-8BIT
        4173 if charset == b"unknown-8bit" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2779 if charset == b"csunknown8bit" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // MNEMONIC
        913 if charset == b"mnemonic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1870 if charset == b"csmnemonic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // MNEM
        279 if charset == b"mnem" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        911 if charset == b"csmnem" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // VISCII
        1711 if charset == b"viscii" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1663 if charset == b"csviscii" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // VIQR
        1874 if charset == b"viqr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1186 if charset == b"csviqr" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),

        // HZ-GB-2312
        1941 if charset == b"hz-gb-2312" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM866
        531 if charset == b"ibm866" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        755 if charset == b"cp866" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        523 if charset == b"866" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1098 if charset == b"csibm866" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM775
        446 if charset == b"ibm775" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        670 if charset == b"cp775" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2058 if charset == b"cspc775baltic" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),

        // IBM00858
        1221 if charset == b"ibm00858" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2038 if charset == b"ccsid00858" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        592 if charset == b"cp00858" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2539 if charset == b"pc-multilingual-850+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1688 if charset == b"csibm00858" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM00924
        733 if charset == b"ibm00924" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1395 if charset == b"ccsid00924" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        802 if charset == b"cp00924" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1594 if charset == b"ebcdic-latin9--euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1045 if charset == b"csibm00924" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01140
        618 if charset == b"ibm01140" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        945 if charset == b"ccsid01140" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        462 if charset == b"cp01140" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1293 if charset == b"ebcdic-us-37+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        595 if charset == b"csibm01140" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01141
        168 if charset == b"ibm01141" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        845 if charset == b"ccsid01141" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        362 if charset == b"cp01141" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1683 if charset == b"ebcdic-de-273+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        495 if charset == b"csibm01141" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01142
        148 if charset == b"ibm01142" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        835 if charset == b"ccsid01142" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        352 if charset == b"cp01142" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2298 if charset == b"ebcdic-dk-277+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1313 if charset == b"ebcdic-no-277+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        485 if charset == b"csibm01142" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01143
        708 if charset == b"ibm01143" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1705 if charset == b"ccsid01143" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1222 if charset == b"cp01143" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1693 if charset == b"ebcdic-fi-278+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1943 if charset == b"ebcdic-se-278+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1355 if charset == b"csibm01143" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01144
        353 if charset == b"ibm01144" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        905 if charset == b"ccsid01144" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        422 if charset == b"cp01144" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2023 if charset == b"ebcdic-it-280+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        555 if charset == b"csibm01144" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01145
        218 if charset == b"ibm01145" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        875 if charset == b"ccsid01145" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        392 if charset == b"cp01145" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1789 if charset == b"ebcdic-es-284+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        525 if charset == b"csibm01145" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01146
        643 if charset == b"ibm01146" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1125 if charset == b"ccsid01146" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        642 if charset == b"cp01146" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1698 if charset == b"ebcdic-gb-285+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        775 if charset == b"csibm01146" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01147
        406 if charset == b"ibm01147" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1215 if charset == b"ccsid01147" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        732 if charset == b"cp01147" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1063 if charset == b"ebcdic-fr-297+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        865 if charset == b"csibm01147" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01148
        1121 if charset == b"ibm01148" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        975 if charset == b"ccsid01148" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        492 if charset == b"cp01148" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1254 if charset == b"ebcdic-international-500+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        625 if charset == b"csibm01148" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM01149
        558 if charset == b"ibm01149" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1575 if charset == b"ccsid01149" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1092 if charset == b"cp01149" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1563 if charset == b"ebcdic-is-871+euro" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1225 if charset == b"csibm01149" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // BIG5-HKSCS
        4842 if charset == b"big5-hkscs" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2537 if charset == b"csbig5hkscs" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // IBM1047
        502 if charset == b"ibm1047" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        401 if charset == b"ibm-1047" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        879 if charset == b"csibm1047" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // PTCP154
        607 if charset == b"ptcp154" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        259 if charset == b"csptcp154" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        530 if charset == b"pt154" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        345 if charset == b"cp154" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        3958 if charset == b"cyrillic-asian" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // AMIGA-1251
        2805 if charset == b"amiga-1251" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1312 if charset == b"ami1251" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2704 if charset == b"amiga1251" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1343 if charset == b"ami-1251" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2546 if charset == b"csamiga1251" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // KOI7-SWITCHED
        2784 if charset == b"koi7-switched" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        2690 if charset == b"cskoi7switched" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // BRF
        448 if charset == b"brf" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        385 if charset == b"csbrf" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // TSCII
        0 if charset == b"tscii" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        87 if charset == b"cstscii" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // CP51932
        1082 if charset == b"cp51932" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1039 if charset == b"cscp51932" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        // WINDOWS-874
        2716 if charset == b"windows-874" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        1442 if charset == b"cswindows874" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),

        // CP50220
        437 if charset == b"cp50220" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
        219 if charset == b"cscp50220" => Some(Box::new(SingleByteDecoder::new(SingleByteDecoder::ISO_8859_2, capacity))),
*/
