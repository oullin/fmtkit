//! `Config::parse` over arbitrary UTF-8: errors, never panics. A config that
//! parses can be hashed, and serialising it back to TOML parses to the same
//! config.

#![no_main]

use std::path::Path;

use fmtkit_config::Config;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let path = Path::new("fmtkit.toml");
    let Ok(config) = Config::parse(text, path) else {
        return;
    };

    let _ = config.hash();
    let _ = config.resolve_jobs(None);

    // Not every value TOML can read is one it can write (an out-of-range
    // `jobs`, say); only a successful write is checked.
    let Ok(written) = toml::to_string(&config) else {
        return;
    };

    match Config::parse(&written, path) {
        Ok(again) => {
            assert_eq!(again, config, "a written config parses differently:\n{written}");
            assert_eq!(again.hash(), config.hash());
        }
        Err(e) => panic!("a written config does not parse: {e}\n{written}"),
    }
});
