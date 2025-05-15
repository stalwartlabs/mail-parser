use std::borrow::Cow;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Html<'x>(Cow<'x, str>);

impl<'x> Html<'x> {
    pub(crate) fn make_owned(self) -> Html<'static> {
        Html(self.0.into_owned().into())
    }
    pub(crate) fn new(html: Cow<'x, str>) -> Html<'x> {
        Html(html)
    }
    /// Access the raw html with the original charset.
    ///
    /// `mail-parser` returns utf-8 strings, so the only correct charset for the html is utf-8. Because html can declare its charset in `<meta>` tags, these may be incorrect after transcoding.
    /// If the charset must be correct call [`Html::fix_charset`] before accessing the html with this method.
    pub fn potentially_wrong_charset(&self) -> &Cow<'x, str> {
        &self.0
    }
    /// Replace charset with `utf-8`.
    ///
    /// This method should be called if the consumer of the html is a standard-conforming browser.
    pub fn fix_charset(&mut self) {
        let mut off = 0;
        let mut first = true;
        let mut found = Vec::with_capacity(2);
        for part in self.0.split("<meta ") {
            if !first {
                let Some((between, _)) = part.split_once('>') else {
                    return;
                };
                for w in between.as_bytes().windows(b"charset".len()) {
                    if w.eq_ignore_ascii_case(b"charset") {
                        found.push((
                            off as isize,
                            (off + "<meta ".len() + between.len() + ">".len()) as isize,
                        ));
                        break;
                    }
                }
                off += "<meta ".len();
            }
            off += part.len();
            first = false;
        }
        let mut deleted: isize = 0;
        let mut first = true;
        for (start, end) in found {
            let mut replace = "";
            if first {
                replace = "<meta charset=utf-8>";
            }
            self.0.to_mut().replace_range(
                (start - deleted) as usize..(end - deleted) as usize,
                replace,
            );
            deleted += end - start - replace.len() as isize;
            first = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(html: &str) -> Cow<'_, str> {
        let mut html = Html(html.into());
        html.fix_charset();
        html.potentially_wrong_charset().clone()
    }

    #[test]
    fn fix_charset() {
        assert_eq!(
            fix("<head><meta cHarSet=Windows-1252></head>"),
            "<head><meta charset=utf-8></head>"
        );

        let fixed = fix("<head><meta cHarSet=\"Windows-1252\"></head>");
        assert_eq!(fixed, "<head><meta charset=utf-8></head>");

        let fixed = fix("<head><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet=Windows-1252\"></head>");
        assert_eq!(fixed, "<head><meta charset=utf-8></head>");

        let fixed = fix("<head><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;></head>");
        assert_eq!(fixed, "<head><meta charset=utf-8></head>");

        let fixed = fix("<head><meta name=\"xxx\"><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;></head>");
        assert_eq!(
            fixed,
            "<head><meta name=\"xxx\"><meta charset=utf-8></head>"
        );

        let fixed = fix("<head><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;><meta name=\"xxx\"></head>");
        assert_eq!(
            fixed,
            "<head><meta charset=utf-8><meta name=\"xxx\"></head>"
        );

        let fixed = fix("<head><meta cHarSet=Windows-1252><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;><meta name=\"xxx\"></head>");
        assert_eq!(
            fixed,
            "<head><meta charset=utf-8><meta name=\"xxx\"></head>"
        );

        let malformed = fix("<head><meta cHarSet=Windows-1252<meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;><meta name=\"xxx\"></head>");
        assert_eq!(
            malformed,
            "<head><meta cHarSet=Windows-1252<meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;><meta name=\"xxx\"></head>"
        );

        let fixed = fix("<metacharset></metacharset>");
        assert_eq!(fixed, "<metacharset></metacharset>");
    }
}
