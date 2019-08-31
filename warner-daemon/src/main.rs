#[macro_use]
extern crate log;

use chrono::{Duration, FixedOffset, TimeZone, Utc};
use config::{Config, File as ConfigFile};
use futures::future::join_all;
use futures::future::Future;
use serde_json::to_string;
use simplelog::{CombinedLogger, Config as LogConfig, LevelFilter, WriteLogger};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::{fs, time as native_time};
use time::Duration as TimeDuration;
use tokio::timer::Delay;

use csv_reader;
use dashboard;
use game_parser;
use twilio;

fn main() {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        LogConfig::default(),
        File::create("mariners_warner.log").unwrap(),
    )])
    .expect("could not initialize logging infrastructure");

    let twilio_config = get_twilio_config();

    let filename = "seattle-mariners-home-schedule.csv";
    let contents = fs::read_to_string(filename)
        .unwrap_or_else(|_| panic!("Something went wrong reading {}", filename));

    let real_rows = csv_reader::read_rows(&contents);

    let fake_start_date_time = Utc::now() + TimeDuration::seconds(5);
    let fake_game = game_parser::Game::PerfectlyScheduledGame {
        start_date_time: fake_start_date_time,
    };

    let mut perfectly_scheduled_games = parse_perfectly_scheduled_games(real_rows);

    let mut games = vec![fake_game];
    games.append(&mut perfectly_scheduled_games);

    record_parsed_games(&games);

    let jobs: Vec<twilio::TwilioResponseFuture> = games
        .iter()
        .filter_map(|game| -> Option<Vec<GameAlert>> { get_times_to_alert(game) })
        .flatten()
        .map(|time_to_alert: GameAlert| -> twilio::TwilioResponseFuture {
            create_alert_job(time_to_alert, twilio_config.clone())
        })
        .collect();

    let all_jobs = join_all(jobs)
        .map(|r| {
            info!("results {:?}", r);
        })
        .map_err(|e| {
            info!("error {:?}", e);
        });

    tokio::run(all_jobs)
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
                    .map(|minutes: i64| -> Vec<GameAlert> {
                        let duration = Duration::minutes(minutes).num_milliseconds();

                        let n_minutes_before = start_date_time.timestamp_millis() - duration;
                        let n_minutes_after = start_date_time.timestamp_millis() + duration;

                        let before_game_purpose =
                            format!("a mariners game is starting in {} minutes", minutes);
                        let after_game_purpose =
                            format!("a mariners game started {} minutes ago", minutes);

                        let mut times_to_go = vec![GameAlert {
                            time_to_alert: n_minutes_after,
                            purpose: after_game_purpose,
                        }];

                        let before_game_alert = GameAlert {
                            time_to_alert: n_minutes_before,
                            purpose: before_game_purpose,
                        };

                        let time_betwen_now_and_alert: i64 =
                            before_game_alert.time_to_alert - Utc::now().timestamp_millis();
                        if time_betwen_now_and_alert <= 0 {
                        } else {
                            times_to_go.push(before_game_alert)
                        }

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

fn create_alert_job(
    game_alert: GameAlert,
    t: twilio::TwilioConfig,
) -> twilio::TwilioResponseFuture {
    let time_to_sleep = native_time::Duration::from_millis(
        (game_alert.time_to_alert - Utc::now().timestamp_millis()) as u64,
    );

    let when = native_time::Instant::now() + time_to_sleep;

    info!(
        "sleeping for {:?} game: {:?} until {:?}",
        time_to_sleep, game_alert.time_to_alert, when
    );

    let delayed_twilio_future = Delay::new(when)
        .map_err(|e| twilio::SMSError::ExecutionError {
            error: e.to_string(),
        })
        .and_then(move |_| {
            let executor = twilio::HTTPExecutor;
            info!("using executor: {}", executor);
            twilio::send_text_message(
                &t.from,
                &t.to,
                &t.twilio_account_id,
                &t.twilio_access_token,
                &game_alert.purpose,
                &executor,
            )
        });

    Box::new(delayed_twilio_future)
}

fn record_parsed_games(games: &[game_parser::Game]) {
    let game_statuses: Vec<dashboard::GameInfo> = games
        .iter()
        .map(|game| dashboard::GameInfo {
            game,
            status: dashboard::WarningStatus::Waiting,
        })
        .collect();

    let s = to_string(&game_statuses).expect("Could not seralize into string");
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
    let mut settings = Config::default();
    settings.merge(ConfigFile::with_name("config")).expect(
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
