use chrono::format::ParseError;
use chrono::prelude::*;
use chrono::Utc;

#[derive(Debug, PartialEq, Clone)]
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
    let start_day_col = match csv_row.get(0) {
        Some(x) => x,
        None => "",
    };
    let start_day = parse_start_day(start_day_col);

    let start_time_col = match csv_row.get(1) {
        Some(x) => x,
        None => "",
    };
    let start_time = parse_start_time(start_time_col);

    match (&start_day, &start_time) {
        (Some(x), Some(y)) => create_perfect_game(x, y),
        (Some(x), _) => Some(Game::GameWithDay {
            start_day: x.to_string(),
        }),
        (_, Some(y)) => Some(Game::GameWithTime {
            start_time: y.to_string(),
        }),
        _ => Some(Game::UnclearGame {
            start_day,
            start_time: start_time.map(|x| x.to_string()),
            start_date_time: None,
        }),
    }
}

fn parse_start_day(c: &str) -> Option<String> {
    if contains_number_like_chars(c) {
        Some(c.to_string())
    } else {
        None
    }
}

fn parse_start_time(c: &str) -> Option<&str> {
    if contains_number_like_chars(c) {
        Some(c)
    } else {
        None
    }
}

fn contains_number_like_chars(c: &str) -> bool {
    c.chars().any(char::is_numeric)
}

fn create_perfect_game(x: &str, y: &str) -> Option<Game> {
    parse_date_time(x, y)
        .ok()
        .map(|dt| Game::PerfectlyScheduledGame {
            start_date_time: dt,
        })
}

fn parse_date_time(date_str: &str, time_str: &str) -> Result<DateTime<Utc>, ParseError> {
    transform_mariners_date_and_time(date_str, time_str).parse::<DateTime<Utc>>()
}

fn transform_mariners_date_and_time(date_str: &str, time_str: &str) -> String {
    let date_parts: Vec<&str> = date_str.split('/').collect();
    let legit: String = match date_parts.as_slice() {
        [month, day, year] => {
            ["20".to_owned() + year, month.to_string(), day.to_string()].join("-")
        }
        _ => date_str.to_owned(),
    };

    let time_and_meridiem = time_str.split(' ').collect::<Vec<&str>>();

    let time_pieces: Vec<u8> = time_and_meridiem[0]
        .split(':')
        .map(|x| {
            let f = x.parse::<u8>();
            f.unwrap()
        })
        .collect();

    let hours_in_24_hr_format = from_12_hr_fmt_to_24_hr_ft(time_pieces[0], time_and_meridiem[1]);

    let iso_8061_ts_start = legit.replace("/", "-") + "T" + &hours_in_24_hr_format;

    let iso_8061_ts_end = ":".to_owned() + &time_pieces[1].to_string() + ":00.0000000-0700";

    iso_8061_ts_start + &iso_8061_ts_end
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
