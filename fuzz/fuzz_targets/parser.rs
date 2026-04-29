#![no_main]

use axhub_helpers::consent::parse_axhub_command;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = parse_axhub_command(input);
});
