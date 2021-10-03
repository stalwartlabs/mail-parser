pub type EmailAddress<'x> = &'x str;
pub type InvalidAddress<'x> = &'x str;

pub struct NamedAddress<'x> {
    name: &'x str,
    email: EmailAddress<'x>,
}

pub enum Address<'x> {
    EmailAddress(EmailAddress<'x>),
    NamedAddress(NamedAddress<'x>),
    InvalidAddress(InvalidAddress<'x>),
}

pub type AddressList<'x> = Vec<Address<'x>>;
