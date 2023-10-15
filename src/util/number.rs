use regex::Regex;

lazy_static::lazy_static! {
    static ref DECIMAL: Regex = Regex::new(r"(?<decimal>\d+(\.\d+)?)").unwrap();
}

pub fn try_parse_number(number: &str) -> Option<f32> {
    if let Some(capture) = DECIMAL.captures(number) {
        let decimal = &capture["decimal"]
            .parse::<f32>()
            .expect("Regex makes sure this is a valid number");
        return Some(*decimal);
    }

    None
}
