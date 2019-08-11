use chrono::prelude::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Game {
    PerfectlyScheduledGame {
        start_date_time: DateTime<Utc>,
    },
    GameWithDay {
        start_day: String,
    },
    GameWithTime {
        start_time: String,
    },
    UnclearGame {
        start_date_time: Option<DateTime<Utc>>,
        start_day: Option<String>,
        start_time: Option<String>,
    },
}

pub fn parse_game_from(csv_row: &[&str]) -> Option<Game> {
    let start_day = csv_row.get(0).and_then(parse_start_day);
    let start_time = csv_row.get(1).and_then(parse_start_time);

    match (&start_day, &start_time) {
        (Some(date), Some(time)) => create_perfect_game(date, time),
        (Some(day), _) => Some(Game::GameWithDay {
            start_day: day.to_string(),
        }),
        (_, Some(t)) => Some(Game::GameWithTime {
            start_time: t.to_string(),
        }),
        _ => Some(Game::UnclearGame {
            start_day,
            start_time: start_time.map(|x| x.to_string()),
            start_date_time: None,
        }),
    }
}

fn parse_start_day(c: &&str) -> Option<String> {
    if contains_number_like_chars(c) {
        Some(c.to_string())
    } else {
        None
    }
}

fn parse_start_time(c: &&str) -> Option<String> {
    if contains_number_like_chars(c) {
        Some(c.to_string())
    } else {
        None
    }
}

fn contains_number_like_chars(c: &str) -> bool {
    c.chars().any(char::is_numeric)
}

fn create_perfect_game(date_str: &str, time_str: &str) -> Option<Game> {
    let iso_8061_ts_start = transform_mariners_date(date_str);

    let iso_8061_ts_end = transform_mariners_time(time_str);

    let date_time = iso_8061_ts_start + "T" + &iso_8061_ts_end;

    date_time
        .parse::<DateTime<Utc>>()
        .ok()
        .map(|dt| Game::PerfectlyScheduledGame {
            start_date_time: dt,
        })
}

fn transform_mariners_date(date_str: &str) -> String {
    let date_parts: Vec<&str> = date_str.split('/').collect();
    match date_parts.as_slice() {
        [month, day, year] => {
            ["20".to_owned() + year, month.to_string(), day.to_string()].join("-")
        }
        _ => date_str.to_owned(),
    }
    .replace("/", "-")
}

fn transform_mariners_time(time_str: &str) -> String {
    let time_and_meridiem: Vec<&str> = time_str.split(' ').collect();

    let time = time_and_meridiem[0];
    let meridiem = time_and_meridiem[1];

    let time_pieces: Vec<u8> = time
        .split(':')
        .map(|x| {
            let f = x.parse::<u8>();
            f.unwrap()
        })
        .collect();

    let hour = time_pieces[0];
    let minute = time_pieces[1];

    let hours_in_24_hr_format = from_12_hr_fmt_to_24_hr_ft(hour, meridiem);

    hours_in_24_hr_format + ":" + &minute.to_string() + ":00.0000000-0700"
}

fn from_12_hr_fmt_to_24_hr_ft(hour: u8, meridian: &str) -> String {
    (hour
        + match meridian {
            "PM" => 12,
            _ => 0,
        })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(
            Ok(Utc.ymd(2015, 5, 15).and_hms(0, 0, 0)),
            "2015-05-15T00:00:00Z".parse::<DateTime<Utc>>()
        );

        let r = parse_game_from(&vec!["11/26/19", "11:30 PM"]);
        assert_eq!(
            Some(Game::PerfectlyScheduledGame {
                start_date_time: Utc.ymd(2019, 11, 27).and_hms(6, 30, 0)
            }),
            r
        );

        assert_eq!(
            Some(Game::GameWithDay {
                start_day: "11/26/19".to_string()
            }),
            parse_game_from(&vec!["11/26/19"])
        );

        assert_eq!(
            Some(Game::GameWithTime {
                start_time: "11:23 PM".to_string()
            }),
            parse_game_from(&vec!["", "11:23 PM"])
        );
    }
}
