use std::borrow::Cow;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Html<'x>(Cow<'x, str>);

impl<'x> Html<'x> {
    pub fn make_owned(self) -> Html<'static> {
        Html(self.0.into_owned().into())
    }
    pub fn new(html: Cow<'x, str>) -> Html<'x> {
        Html(html)
    }
    /// Access the raw html with a potentially wrong charset.
    ///
    /// `mail-parser` only returns utf-8 strings, so the only sensible charset for the html is utf-8. Because html can declare its charset in `<meta>` tags, in the process of transcoding to utf-8 these may be incorrect.
    /// Call [`Html::strip_charset`] before this method if the html will be given to a standard-conforming browser.
    pub fn potentially_wrong_charset(&self) -> &Cow<'x, str> {
        &self.0
    }
    /// Strip charset from html, making it utf-8 by default.
    ///
    /// Call this method if the result is given to a standard-conforming browser.
    pub fn strip_charset(&mut self) {
        let mut off = 0;
        let mut first = true;
        let mut found = None;
        'meta: for part in self.0.split("<meta") {
            if !first {
                let Some((between, _)) = part.split_once('>') else {
                    return;
                };
                for w in between.as_bytes().windows(b"charset".len()) {
                    if w.eq_ignore_ascii_case(b"charset") {
                        found = Some((off, off + "<meta".len() + between.len() + ">".len()));
                        break 'meta;
                    }
                }
                off += "<meta".len();
            }
            off += part.len();
            first = false;
        }
        if let Some((start, end)) = found {
            self.0.to_mut().replace_range(start..end, "");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(html: &str) -> Cow<'_, str> {
        let mut html = Html(html.into());
        html.strip_charset();
        html.potentially_wrong_charset().clone()
    }

    #[test]
    fn strip_charset() {
        assert_eq!(
            strip("<head><meta cHarSet=Windows-1252></head>"),
            "<head></head>"
        );

        let stripped = strip("<head><meta cHarSet=\"Windows-1252\"></head>");
        assert_eq!(stripped, "<head></head>");

        let stripped = strip("<head><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet=Windows-1252\"></head>");
        assert_eq!(stripped, "<head></head>");

        let stripped = strip("<head><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;></head>");
        assert_eq!(stripped, "<head></head>");

        let stripped = strip("<head><meta name=\"xxx\"><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;></head>");
        assert_eq!(stripped, "<head><meta name=\"xxx\"></head>");

        let stripped = strip("<head><meta http-equiv=\"Content-Type\" content=\"text/html; cHarSet = &quot;Windows-1252&quot;><meta name=\"xxx\"></head>");
        assert_eq!(stripped, "<head><meta name=\"xxx\"></head>");
    }
}
