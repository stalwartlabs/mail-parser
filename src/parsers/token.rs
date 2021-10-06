
pub enum TokenPart {
    ByteRange((u32, u32)),
    String(String)
}

pub struct Token {
    items: Vec<TokenPart>
}

impl Token {
    pub fn new() -> Token {
        Token { items: Vec::new() }
    }

    pub fn push_string(&mut self, string: String) {
        self.items.push(TokenPart::String(string));
    }

    pub fn push_bytes(&mut self, from: u32, to: u32) {
        self.items.push(TokenPart::ByteRange((from, to)));
    }



}

