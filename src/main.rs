extern crate chrono;
extern crate config;
extern crate time;

#[macro_use]
extern crate log;
extern crate simplelog;

use chrono::{Duration, FixedOffset, Utc};
use simplelog::*;
use std::collections::HashMap;
use std::fs::File;
use std::{fs, thread, time as native_time};

mod async_latch;
mod csv_reader;
mod game_parser;
mod twilio;

fn main() {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        Config::default(),
        File::create("mariners_warner.log").unwrap(),
    )])
    .expect("could not initialize logging infrastructure");

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

    let filename = "seattle-mariners-home-schedule.csv";

    let contents = fs::read_to_string(filename)
        .unwrap_or_else(|_| panic!("Something went wrong reading {}", filename));

    let real_rows = csv_reader::read_rows(&contents);
    let fake_start_date_time = Utc::now() + time::Duration::seconds(5);
    let fake_game = game_parser::Game::PerfectlyScheduledGame {
        start_date_time: fake_start_date_time,
    };

    let mut perfectly_scheduled_games: Vec<game_parser::Game> = real_rows
        .iter()
        .filter_map(|row| match game_parser::parse_game_from(row) {
            Some(game_parser::Game::PerfectlyScheduledGame { start_date_time }) => {
                Some(game_parser::Game::PerfectlyScheduledGame { start_date_time })
            }
            _ => None,
        })
        .collect();

    perfectly_scheduled_games.push(fake_game);

    let jobs = perfectly_scheduled_games
        .into_iter()
        .filter_map(|game| -> Option<Vec<i64>> {
            match game {
                game_parser::Game::PerfectlyScheduledGame { start_date_time } => {
                    let game_time: i64 =
                        start_date_time.timestamp_millis() - Utc::now().timestamp_millis();
                    if game_time <= 0 {
                        None
                    } else {
                        let mut times_to_alert: Vec<i64> = (1..4)
                            .map(|hours: i64| -> Vec<i64> {
                                let duration = Duration::hours(hours);
                                let times_to_go =
                                    vec![start_date_time - duration, start_date_time + duration];
                                times_to_go
                                    .iter()
                                    .map(|x| x.timestamp_millis())
                                    .collect()
                            })
                            .flatten()
                            .collect();
                        times_to_alert.push(game_time);
                        for time in &times_to_alert {
                            info!(
                                "sleeping for {:?} (u64) for game on {:?} at {:?}",
                                *time as u64,
                                start_date_time
                                    .with_timezone(&FixedOffset::west(7 * 3600))
                                    .date(),
                                start_date_time
                                    .with_timezone(&FixedOffset::west(7 * 3600))
                                    .time()
                            );
                        }
                        Some(times_to_alert)
                    }
                }
                _ => None,
            }
        })
        .flatten()
        .map(|time_to_alert: i64| -> Box<Fn() -> () + Send> {
            let from = from.clone();
            let to = to.clone();
            let twilio_account_id = twilio_account_id.clone();
            let twilio_access_token = twilio_access_token.clone();
            let t = time_to_alert.clone();

            Box::new(move || {
                thread::sleep(native_time::Duration::from_millis(t as u64));
                twilio::send_text_message(
                    &from,
                    &to,
                    &twilio_account_id,
                    &twilio_access_token,
                    &twilio::CommandExecutor,
                )
                .map(|response| info!("{:?}", response))
                .map_err(|e| error!("{:?}", e))
                .ok();
            })
        })
        .collect();

    async_latch::wait(jobs)
}
