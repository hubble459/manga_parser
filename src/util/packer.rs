//! Decoder for Dean Edwards' JavaScript "packer"
//! (`eval(function(p,a,c,k,e,d){...}('payload',62,123,'w0|w1|...'.split('|'),0,{}))`).
//!
//! The payload is plain JS whose identifiers were replaced by base-`a`
//! indices into the `k` word list; unpacking just maps every word back, it
//! runs no JavaScript. Only the common alphabets (`a` <= 62) are supported.

use regex::{Captures, Regex};

use crate::error::ScrapeError;

lazy_static::lazy_static! {
    static ref PACKED: Regex = Regex::new(
        r"(?s)\}\s*\(\s*'((?:[^'\\]|\\.)*)'\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*'((?:[^'\\]|\\.)*)'\.split\('\|'\)"
    ).unwrap();
    static ref WORD: Regex = Regex::new(r"\b\w+\b").unwrap();
}

const ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Unpacks the first packer call found in `text`.
pub fn unpack(text: &str) -> Result<String, ScrapeError> {
    let captures = PACKED
        .captures(text)
        .ok_or_else(|| ScrapeError::WebScrapingError("No packed JavaScript found".to_string()))?;
    let payload = unescape(&captures[1]);
    let radix: usize = captures[2].parse().unwrap_or(0);
    let words: Vec<String> = unescape(&captures[4]).split('|').map(String::from).collect();
    if !(2..=62).contains(&radix) {
        return Err(ScrapeError::WebScrapingError(format!(
            "Unsupported packer radix {radix}"
        )));
    }

    Ok(WORD
        .replace_all(&payload, |word: &Captures| {
            let word = &word[0];
            decode(word, radix)
                .and_then(|index| words.get(index))
                .filter(|replacement| !replacement.is_empty())
                .cloned()
                .unwrap_or_else(|| word.to_string())
        })
        .into_owned())
}

/// A word in packer's base-`radix` alphabet, or `None` when it isn't one.
fn decode(word: &str, radix: usize) -> Option<usize> {
    let digits = &ALPHABET[..radix];
    word.chars().try_fold(0usize, |value, c| {
        let digit = digits.find(c)?;
        value.checked_mul(radix)?.checked_add(digit)
    })
}

/// Undoes the escaping of a single-quoted JS string literal.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod test {
    use super::unpack;

    #[test]
    fn unpacks_words_by_base_index() {
        // "1 0=[\'2\',\'3\']" with words [imgs, var, //a.jpg-ish...]
        let packed = r"<script>eval(function(p,a,c,k,e,d){e=function(c){return c};while(c--){if(k[c]){p=p.replace(new RegExp('\\b'+e(c)+'\\b','g'),k[c])}}return p}('1 0=[\'//4.5/2\',\'//4.5/3\'];',62,6,'newImgs|var|a|b|cdn|com'.split('|'),0,{}))</script>";

        assert_eq!(unpack(packed).unwrap(), "var newImgs=['//cdn.com/a','//cdn.com/b'];");
    }

    #[test]
    fn keeps_words_that_are_empty_or_out_of_range() {
        // index 1 is empty in the word list, and "z" (35) is out of range.
        let packed = r"}('0 1 z',36,3,'x||y'.split('|'),0,{}))";

        assert_eq!(unpack(packed).unwrap(), "x 1 z");
    }

    #[test]
    fn uses_uppercase_digits_above_base_36() {
        // In base 62, "A" is 36 and "10" is 62.
        let mut words = vec![""; 63];
        words[36] = "upper";
        words[62] = "two-digit";
        let packed = format!("}}('A 10',62,63,'{}'.split('|'),0,{{}}))", words.join("|"));

        assert_eq!(unpack(&packed).unwrap(), "upper two-digit");
    }

    #[test]
    fn errors_without_packed_code() {
        assert!(unpack("var x = 1;").is_err());
    }
}
