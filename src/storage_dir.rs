use regex::Regex;

use lazy_static::lazy_static;


lazy_static! {
    static ref DIR_REPLACE_RE: Regex = Regex::new("[^a-zA-Z0-9_-]+").unwrap();
}

pub fn sanitize(original: &str) -> String {
    DIR_REPLACE_RE.replace(original, "-").to_string()
}
