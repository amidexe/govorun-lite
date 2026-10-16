//! Render admitted amounts/dates. This module never chooses a charge or price.
use chrono::{DateTime, FixedOffset, Local};

pub fn money(kopeks: i64) -> String {
    if kopeks % 100 == 0 {
        format!("{} ₽", kopeks / 100)
    } else {
        format!("{},{:02} ₽", kopeks / 100, kopeks % 100)
    }
}
#[derive(Clone, Copy)]
pub struct Dates(pub Option<FixedOffset>);
impl Default for Dates {
    fn default() -> Self {
        Self(None)
    }
}
impl Dates {
    // Substitute device-local time in common copy; never select payment policy.
    pub fn checkout_message(&self, window: &serde_json::Value) -> Result<Option<String>, &'static str> {
        let Some(message) = window["message"].as_str() else { return Ok(None); };
        if !message.contains("{firstChargeDate}") && !message.contains("{firstChargeTime}") {
            return Ok(Some(message.to_owned()));
        }
        let source = window["outcome"]["firstChargeAt"].as_str();
        let date = self.date(source, false, false, false).ok_or("invalid_billing")?;
        let moment = self.date(source, false, true, false).ok_or("invalid_billing")?;
        let time = moment.rsplit_once(", ").ok_or("invalid_billing")?.1;
        Ok(Some(message.replace("{firstChargeDate}", &date).replace("{firstChargeTime}", time)))
    }
    pub fn date(&self, value: Option<&str>, year: bool, time: bool, long: bool) -> Option<String> {
        let parsed = DateTime::parse_from_rfc3339(value?).ok()?;
        let date = match self.0 {
            Some(offset) => parsed.with_timezone(&offset),
            None => parsed.with_timezone(&Local).fixed_offset(),
        };
        let month = [
            "января",
            "февраля",
            "марта",
            "апреля",
            "мая",
            "июня",
            "июля",
            "августа",
            "сентября",
            "октября",
            "ноября",
            "декабря",
        ];
        use chrono::Datelike;
        let mut text = format!("{} {}", date.day(), month[date.month0() as usize]);
        if year {
            text.push_str(&format!(
                " {}{}",
                date.year(),
                if long { " г." } else { "" }
            ));
        }
        if time {
            text.push_str(&format!(", {}", date.format("%H:%M")));
        }
        Some(text)
    }
}
pub fn delay(seconds: u64) -> String {
    fn plural(n: u64, words: [&str; 3]) -> &str {
        words[if (11..=14).contains(&(n % 100)) {
            2
        } else if n % 10 == 1 {
            0
        } else if (2..=4).contains(&(n % 10)) {
            1
        } else {
            2
        }]
    }
    let mut parts = vec![];
    let hours = seconds / 3600;
    let minutes = seconds % 3600 / 60;
    if hours > 0 {
        parts.push(format!(
            "{hours} {}",
            plural(hours, ["час", "часа", "часов"])
        ));
    }
    if minutes > 0 {
        parts.push(format!(
            "{minutes} {}",
            plural(minutes, ["минуту", "минуты", "минут"])
        ));
    }
    parts.join(" ")
}
