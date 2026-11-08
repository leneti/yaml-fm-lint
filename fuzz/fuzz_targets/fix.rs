//! `fix` keeps its promises (spec §12) on any input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;
use yaml_fm_lint_core::invariants::check_fix;
use yaml_fm_lint_core::{Config, Linter};

fn linter() -> &'static Linter {
    static LINTER: OnceLock<Linter> = OnceLock::new();
    LINTER
        .get_or_init(|| Linter::new(Config::default(), None).expect("the default config is valid"))
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Err(message) = check_fix(linter(), text) {
        panic!("{message}");
    }
});
