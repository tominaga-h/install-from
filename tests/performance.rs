//! Performance guard: SPEC.md §9.10 — total runtime per invocation
//! < 10 ms. Soft assertion only (`eprintln!` warning, no failure) so a
//! loaded/throttled CI runner never turns this into a false-positive
//! failure; the real signal (no subprocess) is checked separately in
//! `no_subprocess.rs`.

mod common;

use common::Fixture;
use std::time::Instant;

const RUNS: u32 = 50;
const SOFT_LIMIT_MS: u128 = 10;

#[test]
fn fifty_runs_average_under_10ms_soft() {
    let fixture = Fixture::new();
    fixture.homebrew("/usr/local", "jq", "1.7.1");

    // Warm the OS file cache / page the binary in before timing.
    fixture.command().arg("jq").assert().success();

    let start = Instant::now();
    for _ in 0..RUNS {
        fixture.command().arg("jq").assert().success();
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() / u128::from(RUNS);

    if avg_ms >= SOFT_LIMIT_MS {
        eprintln!(
            "WARNING: install-from averaged {avg_ms}ms per invocation over {RUNS} runs \
             (soft target: <{SOFT_LIMIT_MS}ms). This does not fail the build; if it's \
             consistently over, something regressed (SPEC.md §9.10)."
        );
    }
}
