use game_parser;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum WarningStatus {
    Waiting,
    Completed,
    Error,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct GameInfo<'a> {
    pub game: &'a game_parser::Game,
    pub status: WarningStatus,
}
