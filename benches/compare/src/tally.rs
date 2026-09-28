use std::{hint::black_box, ops::AddAssign};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    pub digest: u64,
    pub messages: u64,
    pub parts: u64,
    pub leaves: u64,
    pub text_leaves: u64,
    pub decoded_bytes: u64,
    pub text_bytes: u64,
    pub failures: u64,
    pub subject: u64,
    pub from: u64,
    pub date: u64,
    pub message_id: u64,
}

impl Tally {
    pub fn add(&mut self, value: u64) {
        self.digest = self.digest.rotate_left(5) ^ value;
    }

    pub fn bytes(&mut self, bytes: &[u8]) {
        self.add(black_box(bytes).len() as u64);
    }

    pub fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
    }

    pub fn maybe_text(&mut self, text: Option<&str>) {
        if let Some(text) = text {
            self.text(text);
        }
    }

    pub fn content_type(&mut self, ctype: &str, subtype: Option<&str>) {
        self.text(ctype);
        self.maybe_text(subtype);
    }

    pub fn leaf(&mut self, bytes: &[u8], text: bool) {
        let len = black_box(bytes).len() as u64;
        self.leaves += 1;
        self.decoded_bytes += len;
        if text {
            self.text_leaves += 1;
            self.text_bytes += len;
        }
        self.add(len);
    }

    pub fn fields(&self) -> [u64; 12] {
        [
            self.digest,
            self.messages,
            self.parts,
            self.leaves,
            self.text_leaves,
            self.decoded_bytes,
            self.text_bytes,
            self.failures,
            self.subject,
            self.from,
            self.date,
            self.message_id,
        ]
    }

    pub fn from_fields(fields: [u64; 12]) -> Tally {
        let [
            digest,
            messages,
            parts,
            leaves,
            text_leaves,
            decoded_bytes,
            text_bytes,
            failures,
            subject,
            from,
            date,
            message_id,
        ] = fields;
        Tally {
            digest,
            messages,
            parts,
            leaves,
            text_leaves,
            decoded_bytes,
            text_bytes,
            failures,
            subject,
            from,
            date,
            message_id,
        }
    }

    pub fn finish(&self) -> u64 {
        black_box(
            self.digest
                ^ self.parts.rotate_left(8)
                ^ self.leaves.rotate_left(16)
                ^ self.decoded_bytes.rotate_left(24)
                ^ self.messages.rotate_left(40),
        )
    }
}

impl AddAssign for Tally {
    fn add_assign(&mut self, other: Tally) {
        self.digest = self.digest.wrapping_add(other.digest);
        self.messages += other.messages;
        self.parts += other.parts;
        self.leaves += other.leaves;
        self.text_leaves += other.text_leaves;
        self.decoded_bytes += other.decoded_bytes;
        self.text_bytes += other.text_bytes;
        self.failures += other.failures;
        self.subject += other.subject;
        self.from += other.from;
        self.date += other.date;
        self.message_id += other.message_id;
    }
}
