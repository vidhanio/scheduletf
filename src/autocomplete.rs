use std::sync::LazyLock;

use paste::paste;
use regex::Regex;
use time::{Date, Duration, OffsetDateTime, Time, macros::time};

use crate::utils::OffsetDateTimeEtExt;

pub fn split_datetime_query(query: &str) -> (String, String, String) {
    static REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^([a-z]+)?\s*(\d[a-z0-9]*)?$").unwrap());

    let query = query.trim().to_ascii_lowercase();

    if let Some(captures) = REGEX.captures(&query) {
        let day = captures.get(1).map_or("", |m| m.as_str()).into();
        let time = captures.get(2).map_or("", |m| m.as_str()).into();

        (query, day, time)
    } else {
        (query, String::new(), String::new())
    }
}

pub fn day_aliases(date: Date) -> &'static [&'static str] {
    macro_rules! aliases {
            ($($weekday:ident),*) => {
                paste! {
                    let now_date = OffsetDateTime::now_et().date();

                    match (
                        date.weekday(),
                        date == now_date,
                        date == now_date.next_day().unwrap(),
                    ) {
                        $(
                            (time::Weekday::$weekday, true, false) => {
                                &[stringify!([<$weekday:lower>]), "today", "tdy"]
                            }
                            (time::Weekday::$weekday, false, true) => {
                                &[stringify!([<$weekday:lower>]), "tomorrow", "tmrw"]
                            }
                            (time::Weekday::$weekday, false, false) => {
                                &[stringify!([<$weekday:lower>])]
                            }
                        )*
                        _ => &[]
                    }
                }
            };
        }

    aliases! {
        Sunday,
        Monday,
        Tuesday,
        Wednesday,
        Thursday,
        Friday,
        Saturday
    }
}

pub const fn time_aliases(time: Time) -> &'static [&'static str] {
    macro_rules! aliases {
            ($($hour:literal),*) => {
                paste! {
                    let hour_24 = time.hour();
                    let minute = time.minute();
                    let hour = if hour_24 == 0 {
                        12
                    } else if hour_24 > 12 {
                        hour_24 - 12
                    } else {
                        hour_24
                    };

                    match (hour, hour_24 >= 12, minute) {
                        $(
                            ($hour, false, 0) => {
                                &[
                                    concat!($hour, ":00 am"),
                                    concat!($hour, ":00am"),
                                    concat!($hour, ":00 a.m."),
                                    concat!($hour, ":00a.m."),

                                    concat!($hour, "00 am"),
                                    concat!($hour, "00am"),
                                    concat!($hour, "00 a.m."),
                                    concat!($hour, "00a.m."),

                                    concat!($hour, " am"),
                                    concat!($hour, "am"),
                                    concat!($hour, " a.m."),
                                    concat!($hour, "a.m."),
                                ]
                            }
                            ($hour, false, 30) => {
                                &[
                                    concat!($hour, ":30 am"),
                                    concat!($hour, ":30am"),
                                    concat!($hour, ":30 a.m."),
                                    concat!($hour, ":30a.m."),

                                    concat!($hour, "30 am"),
                                    concat!($hour, "30am"),
                                    concat!($hour, "30 a.m."),
                                    concat!($hour, "30a.m."),
                                ]
                            }
                            ($hour, true, 0) => {
                                &[
                                    concat!($hour, ":00 pm"),
                                    concat!($hour, ":00pm"),
                                    concat!($hour, ":00 p.m."),
                                    concat!($hour, ":00p.m."),

                                    concat!($hour, "00 pm"),
                                    concat!($hour, "00pm"),
                                    concat!($hour, "00 p.m."),
                                    concat!($hour, "00p.m."),

                                    concat!($hour, " pm"),
                                    concat!($hour, "pm"),
                                    concat!($hour, " p.m."),
                                    concat!($hour, "p.m."),
                                ]
                            }
                            ($hour, true, 30) => {
                                &[
                                    concat!($hour, ":30 pm"),
                                    concat!($hour, ":30pm"),
                                    concat!($hour, ":30 p.m."),
                                    concat!($hour, ":30p.m."),

                                    concat!($hour, "30 pm"),
                                    concat!($hour, "30pm"),
                                    concat!($hour, "30 p.m."),
                                    concat!($hour, "30p.m."),
                                ]
                            }
                        )*
                        _ => &[]
                    }
                }
            };
        }

    aliases! {
        1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 11, 12
    }
}

pub fn day_choices() -> impl Iterator<Item = (Date, &'static [&'static str])> {
    (0..=7).map(move |i| {
        let date = OffsetDateTime::now_et().date() + Duration::days(i);

        (date, day_aliases(date))
    })
}

pub const DEFAULT_TIME_CHOICES: [Time; 3] = [time!(20:30), time!(21:30), time!(22:30)];

pub static TIME_CHOICES: LazyLock<Vec<(Time, &'static [&'static str])>> = LazyLock::new(|| {
    (0..24)
        .flat_map(move |hour| {
            [0, 30].into_iter().map(move |minute| {
                let time = Time::from_hms(hour, minute, 0).unwrap();
                (time, time_aliases(time))
            })
        })
        .collect()
});
