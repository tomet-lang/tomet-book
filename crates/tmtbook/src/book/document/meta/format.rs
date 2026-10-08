use regex::Regex;
use std::sync::LazyLock;

// Compiled once: this runs per document, in parallel.
static ISO_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^(\d{4})-(\d{2})-(\d{2})"#).unwrap());

pub fn format_chip_value(val: &str, format: Option<&str>, lang: Option<&str>) -> String {
    if format == Some("birthday")
        && let Some(caps) = ISO_DATE_RE.captures(val)
    {
        let m: u32 = caps[2].parse().unwrap_or(0);
        let d: u32 = caps[3].parse().unwrap_or(0);
        if let Some(l) = lang
            && crate::config::is_english(l)
        {
            const MONTHS_EN: [&str; 12] = [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ];
            if (1..=12).contains(&m) {
                return format!("{} {d}", MONTHS_EN[(m - 1) as usize]);
            }
        }
        return format!("{m}月{d}日");
    }
    val.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn birthday_format_shortens_an_iso_date() {
        assert_eq!(
            format_chip_value("2001-04-09", Some("birthday"), None),
            "4月9日"
        );
        assert_eq!(
            format_chip_value("2001-04-09T00:00:00", Some("birthday"), None),
            "4月9日"
        );
        assert_eq!(
            format_chip_value("2001-04-09", Some("birthday"), Some("en")),
            "April 9"
        );
        assert_eq!(
            format_chip_value("2001-12-25", Some("birthday"), Some("en-US")),
            "December 25"
        );
    }

    #[test]
    fn birthday_format_leaves_unparsable_values_alone() {
        assert_eq!(
            format_chip_value("春ごろ", Some("birthday"), None),
            "春ごろ"
        );
        assert_eq!(format_chip_value("2001-04-09", None, None), "2001-04-09");
    }
}
