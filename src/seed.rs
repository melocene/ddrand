use chrono::Datelike;
use rand::{Rng, distr::Alphanumeric, rng, rngs::StdRng};
use rand_pcg::Pcg64;
use rand_seeder::Seeder;
use tracing::debug;

pub fn create_rng(seed: &str) -> StdRng {
    Seeder::from(seed).into_rng()
}

pub fn generate_seed() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(32)
        .collect::<String>()
}

pub fn generate_weekly_seed() -> String {
    let current_date = chrono::Local::now().date_naive();
    let current_week = current_date.iso_week().week0();
    debug!("Current week: {}", current_week);
    let week_seed = format!("{}{}seedoftheweek", current_date.year(), current_week);
    debug!("Weekly base seed: {}", &week_seed);
    let week_rng: Pcg64 = Seeder::from(week_seed).into_rng();
    let wseed = week_rng
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(32)
        .collect::<String>();
    debug!("Seed of week {}: {}", current_week, wseed);

    wseed
}

// #[cfg(test)]
// This file does not require testing since it is a simple wrapper around the rand library.
// Refer to https://docs.rs/rand/latest/rand/ for library specific documentation and tests.
