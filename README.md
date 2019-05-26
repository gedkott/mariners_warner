# mariners_warner
warns me when the mariners are having a home game so I can get out of SODO 

# disclaimer
this was a toy project. it is dangerous and not safe to use.

# Running
use `rustup`, `cargo` 

`git clone`, `cargo build`

`cp` `config.template.toml` as `config.toml` and fill in the `to` and `from` phone number keys for twilio 

the phone numbers should just be the number (e.g. `"1234567890"`) and not include country code (for twilio)

phone numbers are assumed to be US numbers; no non-US phone number support

fill in the twilio account ID and access token as well

with config ready, `cargo run`
