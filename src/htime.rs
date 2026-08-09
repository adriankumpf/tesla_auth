use std::fmt;
use std::time;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// Wraps a [`time::Duration`] to display it as e.g. `1 day 23 hours 59 minutes`.
#[derive(Debug, Clone)]
pub struct Duration(time::Duration);

impl From<time::Duration> for Duration {
    fn from(duration: time::Duration) -> Self {
        Self(duration)
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut remaining = self.0.as_secs();

        if remaining < MINUTE {
            return f.write_str("less than a minute");
        }

        let mut separator = "";

        for (secs, unit) in [(DAY, "day"), (HOUR, "hour"), (MINUTE, "minute")] {
            let units = remaining / secs;
            remaining %= secs;

            if units == 0 {
                continue;
            }

            let plural = if units == 1 { "" } else { "s" };

            write!(f, "{separator}{units} {unit}{plural}")?;
            separator = " ";
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations_in_words() {
        let pp = |secs| Duration::from(time::Duration::from_secs(secs)).to_string();

        assert_eq!(pp(0), "less than a minute");
        assert_eq!(pp(MINUTE - 1), "less than a minute");
        assert_eq!(pp(30), "less than a minute");
        assert_eq!(pp(MINUTE), "1 minute");

        assert_eq!(pp(DAY / 2), "12 hours");
        assert_eq!(pp(DAY), "1 day");
        assert_eq!(pp(10 * DAY), "10 days");

        assert_eq!(pp(DAY + MINUTE - 1), "1 day");
        assert_eq!(pp(DAY + MINUTE), "1 day 1 minute");
        assert_eq!(pp(DAY - 1), "23 hours 59 minutes");

        assert_eq!(pp(2 * DAY - 1), "1 day 23 hours 59 minutes");
    }
}
