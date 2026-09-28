use crate::{
    corpus::Corpus,
    ffi::{self, Native},
    impls,
    tally::Tally,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Workload {
    Structure,
    Headers,
    Full,
}

impl Workload {
    pub const ALL: [Workload; 3] = [Workload::Structure, Workload::Headers, Workload::Full];

    pub fn name(self) -> &'static str {
        match self {
            Workload::Structure => "structure",
            Workload::Headers => "headers",
            Workload::Full => "full",
        }
    }

    pub fn from_name(name: &str) -> Option<Workload> {
        Workload::ALL
            .into_iter()
            .find(|workload| workload.name() == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Implementation {
    MailParser,
    MailParser011,
    Mailparse,
    Gmime,
    Vmime,
    Libetpan,
    Dovecot,
}

impl Implementation {
    pub const ALL: [Implementation; 7] = [
        Implementation::MailParser,
        Implementation::MailParser011,
        Implementation::Mailparse,
        Implementation::Gmime,
        Implementation::Vmime,
        Implementation::Libetpan,
        Implementation::Dovecot,
    ];

    pub const REFERENCE: Implementation = Implementation::MailParser;

    pub fn name(self) -> &'static str {
        match self {
            Implementation::MailParser => "mail-parser-1.0",
            Implementation::MailParser011 => "mail-parser-0.11",
            Implementation::Mailparse => "mailparse",
            Implementation::Gmime => "gmime",
            Implementation::Vmime => "vmime",
            Implementation::Libetpan => "libetpan",
            Implementation::Dovecot => "dovecot",
        }
    }

    pub fn from_name(name: &str) -> Option<Implementation> {
        Implementation::ALL
            .into_iter()
            .find(|implementation| implementation.name() == name)
    }

    pub fn is_native(self) -> bool {
        self.native().is_some()
    }

    fn native(self) -> Option<Native> {
        match self {
            Implementation::Gmime => Some(Native::Gmime),
            Implementation::Vmime => Some(Native::Vmime),
            Implementation::Libetpan => Some(Native::Libetpan),
            Implementation::Dovecot => Some(Native::Dovecot),
            Implementation::MailParser
            | Implementation::MailParser011
            | Implementation::Mailparse => None,
        }
    }
}

#[derive(Default)]
pub struct Parsers {
    v1: mail_parser::MessageParser,
    v011: mail_parser_011::MessageParser,
}

impl Parsers {
    pub fn run(
        &self,
        implementation: Implementation,
        workload: Workload,
        raw: &[u8],
        tally: &mut Tally,
    ) -> bool {
        match implementation {
            Implementation::MailParser => impls::v1::run(&self.v1, workload, raw, tally),
            Implementation::MailParser011 => impls::v011::run(&self.v011, workload, raw, tally),
            Implementation::Mailparse => impls::mailparse::run(workload, raw, tally),
            Implementation::Gmime
            | Implementation::Vmime
            | Implementation::Libetpan
            | Implementation::Dovecot => implementation
                .native()
                .is_some_and(|native| ffi::run(native, workload, raw, tally)),
        }
    }

    pub fn digest(&self, implementation: Implementation, workload: Workload, raw: &[u8]) -> u64 {
        let mut tally = Tally::default();
        self.run(implementation, workload, raw, &mut tally);
        tally.finish()
    }

    pub fn run_corpus(
        &self,
        implementation: Implementation,
        workload: Workload,
        corpus: &Corpus,
    ) -> u64 {
        corpus.samples.iter().fold(0u64, |digest, sample| {
            digest.wrapping_add(self.digest(implementation, workload, &sample.bytes))
        })
    }
}
