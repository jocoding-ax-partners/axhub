#![no_main]

use axhub_helpers::routing::{axhub_keyword_present, deploy_create_intent_present};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = axhub_keyword_present(input);
    let _ = deploy_create_intent_present(input);
});
