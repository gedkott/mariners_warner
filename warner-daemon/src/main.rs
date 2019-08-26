extern crate chrono;
extern crate config;
extern crate time;

#[macro_use]
extern crate log;
extern crate simplelog;

use chrono::{Duration, FixedOffset, TimeZone, Utc};
use simplelog::{CombinedLogger, Config, LevelFilter, WriteLogger};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::{fs, thread, time as native_time};

use async_latch;
use csv_reader;
use dashboard;
use game_parser;
use twilio;

fn main() {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        Config::default(),
        File::create("mariners_warner.log").unwrap(),
    )])
    .expect("could not initialize logging infrastructure");

    let twilio_config = get_twilio_config();

    let filename = "seattle-mariners-home-schedule.csv";
    let contents = fs::read_to_string(filename)
        .unwrap_or_else(|_| panic!("Something went wrong reading {}", filename));

    let real_rows = csv_reader::read_rows(&contents);

    let fake_start_date_time = Utc::now() + time::Duration::seconds(5);
    let fake_game = game_parser::Game::PerfectlyScheduledGame {
        start_date_time: fake_start_date_time,
    };

    let mut perfectly_scheduled_games = parse_perfectly_scheduled_games(real_rows);

    let mut games = vec![fake_game];
    games.append(&mut perfectly_scheduled_games);

    record_parsed_games(&games);

    let jobs: Vec<Box<Fn() -> () + Send>> = games
        .iter()
        .filter_map(|game| -> Option<Vec<GameAlert>> { get_times_to_alert(game) })
        .flatten()
        .map(|time_to_alert: GameAlert| -> Box<Fn() -> () + Send> {
            create_alert_job(time_to_alert, twilio_config.clone())
        })
        .collect();

    async_latch::wait(jobs)
}

struct GameAlert {
    purpose: String,
    time_to_alert: i64,
}

fn get_times_to_alert(game: &game_parser::Game) -> Option<Vec<GameAlert>> {
    match game {
        game_parser::Game::PerfectlyScheduledGame { start_date_time } => {
            let time_betwen_now_and_game: i64 =
                start_date_time.timestamp_millis() - Utc::now().timestamp_millis();
            if time_betwen_now_and_game <= 0 {
                None
            } else {
                let mut game_alerts: Vec<GameAlert> = (1..4)
                    .map(|hours: i64| -> Vec<GameAlert> {
                        let duration = Duration::hours(hours).num_milliseconds();

                        let n_hours_before = start_date_time.timestamp_millis() - duration;
                        let n_hours_after = start_date_time.timestamp_millis() + duration;

                        let before_game_purpose =
                            format!("a mariners game is starting in {} hours", hours);
                        let after_game_purpose =
                            format!("a mariners game ended {} hours ago", hours);

                        let times_to_go = vec![
                            GameAlert {
                                time_to_alert: n_hours_before,
                                purpose: before_game_purpose,
                            },
                            GameAlert {
                                time_to_alert: n_hours_after,
                                purpose: after_game_purpose,
                            },
                        ];
                        times_to_go.into_iter().collect()
                    })
                    .flatten()
                    .collect();
                game_alerts.push(GameAlert {
                    time_to_alert: start_date_time.timestamp_millis(),
                    purpose: "a mariners game is starting now".to_string(),
                });
                for game_alert in &game_alerts {
                    info!(
                        "going to text at {:?} for game on {:?} at {:?}",
                        Utc.timestamp_millis(game_alert.time_to_alert)
                            .with_timezone(&FixedOffset::west(7 * 3600))
                            .to_rfc2822(),
                        start_date_time
                            .with_timezone(&FixedOffset::west(7 * 3600))
                            .date(),
                        start_date_time
                            .with_timezone(&FixedOffset::west(7 * 3600))
                            .time()
                    );
                }
                Some(game_alerts)
            }
        }
        _ => None,
    }
}

fn create_alert_job(game_alert: GameAlert, t: twilio::TwilioConfig) -> Box<Fn() -> () + Send> {
    let time_to_sleep = native_time::Duration::from_millis(
        (game_alert.time_to_alert - Utc::now().timestamp_millis()) as u64,
    );

    info!("sleeping for {:?}", time_to_sleep,);

    Box::new(move || {
        thread::sleep(time_to_sleep);
        twilio::send_text_message(
            &t.from,
            &t.to,
            &t.twilio_account_id,
            &t.twilio_access_token,
            &game_alert.purpose,
            &twilio::CommandExecutor,
        )
        .map(|response| info!("{:?}", response))
        .map_err(|e| error!("{:?}", e))
        .ok();
    })
}

fn record_parsed_games(games: &[game_parser::Game]) {
    let game_statuses: Vec<dashboard::GameInfo> = games
        .iter()
        .map(|game| dashboard::GameInfo {
            game,
            status: dashboard::WarningStatus::Waiting,
        })
        .collect();

    let s = serde_json::to_string(&game_statuses).expect("Could not seralize into string");
    let mut file = File::create("foo.txt").expect("Could not open file to write");
    file.write_all(s.as_bytes())
        .expect("Could not open write string to file")
}

fn parse_perfectly_scheduled_games(raw_csv_rows: Vec<Vec<&str>>) -> Vec<game_parser::Game> {
    raw_csv_rows
        .iter()
        .filter_map(|row| match game_parser::parse_game_from(row.as_slice()) {
            Some(game_parser::Game::PerfectlyScheduledGame { start_date_time }) => {
                Some(game_parser::Game::PerfectlyScheduledGame { start_date_time })
            }
            _ => None,
        })
        .collect()
}

fn get_twilio_config() -> twilio::TwilioConfig {
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("config")).expect(
        "you must supply a config file named config.toml matching config.template.toml's structure",
    );
    let app_config = settings.try_into::<HashMap<String, String>>().expect(
        "you must supply a config file named config.toml matching config.template.toml's structure",
    );

    let from = app_config.get("from").expect("config.toml must define a from phone number in the form \"\\d\\d\\d\\d\\d\\d\\d\\d\\d\\d\"");
    let to = app_config.get("to").expect(
        "config.toml must define a to phone number in the form \"\\d\\d\\d\\d\\d\\d\\d\\d\\d\\d\"",
    );
    let twilio_account_id = app_config
        .get("twilio_account_id")
        .expect("config.toml must define a twilio_account_id");
    let twilio_access_token = app_config
        .get("twilio_access_token")
        .expect("config.toml must define a twilio_access_token");

    twilio::TwilioConfig {
        from: from.clone(),
        to: to.clone(),
        twilio_access_token: twilio_access_token.clone(),
        twilio_account_id: twilio_account_id.clone(),
    }
}
