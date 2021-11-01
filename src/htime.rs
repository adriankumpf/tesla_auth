use std::fmt;
use std::time;

/// A wrapper type that allows to display a Duration
#[derive(Debug, Clone)]
pub struct Duration(time::Duration);

impl From<time::Duration> for Duration {
    fn from(duration: time::Duration) -> Self {
        Self(duration)
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", pretty_print(&self.0))
    }
}

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

fn pretty_print(d: &time::Duration) -> String {
    let mut d = d.as_secs();
    let mut out = vec![];

    for (secs, suffix) in [(DAY, "day"), (HOUR, "hour"), (MINUTE, "minute")] {
        if d < secs {
            continue;
        }

        let units = d / secs;

        out.push(if units == 1 {
            format!("{} {}", units, suffix)
        } else {
            format!("{} {}s", units, suffix)
        });

        d = d
            .checked_sub(units.checked_mul(secs).expect("overflow"))
            .expect("overflow");
    }

    out.join(" ")
}

#[test]
fn test_pretty_print() {
    let pp = |secs| pretty_print(&time::Duration::from_secs(secs));

    assert_eq!(pp(0), "");
    assert_eq!(pp(MINUTE - 1), "");
    assert_eq!(pp(MINUTE), "1 minute");

    assert_eq!(pp(DAY / 2), "12 hours");
    assert_eq!(pp(DAY), "1 day");
    assert_eq!(pp(10 * DAY), "10 days");

    assert_eq!(pp(DAY + MINUTE - 1), "1 day");
    assert_eq!(pp(DAY + MINUTE), "1 day 1 minute");
    assert_eq!(pp(DAY - 1), "23 hours 59 minutes");

    assert_eq!(pp(2 * DAY - 1), "1 day 23 hours 59 minutes");
}
