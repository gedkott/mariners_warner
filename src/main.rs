extern crate chrono;
extern crate config;
extern crate time;

use chrono::{FixedOffset, Utc};
use std::collections::HashMap;
use std::{fs, sync, thread, time as native_time};

mod async_latch;
mod csv_reader;
mod game_parser;
mod twilio;

fn main() {
    let mut settings = config::Config::default();

    settings.merge(config::File::with_name("config")).unwrap();

    let app_config = settings.try_into::<HashMap<String, String>>().unwrap();

    // Print out our settings (as a HashMap)
    println!("{:?}", app_config);

    let from = sync::Arc::new(sync::Mutex::new(app_config.get("from").unwrap().clone()));
    let to = sync::Arc::new(sync::Mutex::new(app_config.get("to").unwrap().clone()));
    let twilio_account_id = sync::Arc::new(sync::Mutex::new(
        app_config.get("twilio_account_id").unwrap().clone(),
    ));
    let twilio_access_token = sync::Arc::new(sync::Mutex::new(
        app_config.get("twilio_access_token").unwrap().clone(),
    ));

    let filename = "seattle-mariners-home-schedule.csv";

    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");

    let real_rows = csv_reader::read_rows(&contents);
    let fake_start_date_time = Utc::now() + time::Duration::seconds(5);
    let fake_sleep_time = fake_start_date_time.timestamp_millis() - Utc::now().timestamp_millis();
    let fake_game = (
        game_parser::Game::PerfectlyScheduledGame {
            start_date_time: fake_start_date_time,
        },
        fake_sleep_time,
        from.clone(),
        to.clone(),
        twilio_account_id.clone(),
        twilio_access_token.clone(),
    );

    let mut games: Vec<(
        game_parser::Game,
        i64,
        sync::Arc<sync::Mutex<String>>,
        sync::Arc<sync::Mutex<String>>,
        sync::Arc<sync::Mutex<String>>,
        sync::Arc<sync::Mutex<String>>,
    )> = real_rows
        .iter()
        .filter_map(|row| {
            let game = game_parser::parse_game_from(row);
            let r = match game {
                Some(game_parser::Game::PerfectlyScheduledGame { start_date_time }) => {
                    let time_to_sleep =
                        start_date_time.timestamp_millis() - Utc::now().timestamp_millis();
                    if time_to_sleep > 0 {
                        let from = from.clone();
                        let to = to.clone();
                        let twilio_account_id = twilio_account_id.clone();
                        let twilio_access_token = twilio_access_token.clone();
                        Some((
                            game.unwrap().clone(),
                            time_to_sleep,
                            from,
                            to,
                            twilio_account_id,
                            twilio_access_token,
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            r
        })
        .collect();

    games.push(fake_game);

    let jobs = games.into_iter().filter_map(
            |(game, time_to_sleep, from, to, twilio_account_id, twilio_access_token)| -> Option<sync::Arc<Fn() -> () + Send + Sync>> {
                match game {
                    game_parser::Game::PerfectlyScheduledGame { start_date_time } => {
                        Some(sync::Arc::new(move || {
                            println!(
                                "sleeping for {:?} (u64) for game on {:?} at {:?}",
                                time_to_sleep as u64,
                                start_date_time
                                    .with_timezone(&FixedOffset::west(7 * 3600))
                                    .date(),
                                start_date_time
                                    .with_timezone(&FixedOffset::west(7 * 3600))
                                    .time()
                            );
                            thread::sleep(native_time::Duration::from_millis(time_to_sleep as u64));
                            println!("Game starting");

                            let r = twilio::send_text_message(&from.lock().unwrap(), &to.lock().unwrap(), &twilio_account_id.lock().unwrap(), &twilio_access_token.lock().unwrap());
                            println!("{:?}", r)
                        }))
                    }
                    _ => None,
                }
            },
        )
        .collect();

    async_latch::wait(jobs)
}
