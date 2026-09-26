//! Balance runner: play N days for S seeds with the autoplay bot and print a table.
//!
//! Usage: `cargo run -p proof_tools --bin autoplay -- [days] [seeds]`

use proof_core::sim::Bot;
use proof_core::state::GameState;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let days: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);
    let seeds: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
    let mut totals = vec![(0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64); days as usize];
    let mut unlock_days: Vec<Vec<u32>> = Vec::new();
    for seed in 0..seeds {
        let mut g = GameState::new(seed);
        let mut bot = Bot::new(seed);
        let mut owned = g.unlocked.len();
        let mut seen = Vec::new();
        for t in totals.iter_mut() {
            let log = bot.play_day(&mut g);
            t.0 += log.gestures as u64;
            t.1 += log.earned as u64;
            t.2 += log.level as u64;
            t.3 += log.served as u64;
            t.4 += log.visitors as u64;
            t.5 += log.loved as u64;
            t.6 += log.baked as u64;
            if g.unlocked.len() > owned {
                seen.push(log.day);
                owned = g.unlocked.len();
            }
        }
        unlock_days.push(seen);
    }
    let n = seeds as f64;
    println!("day | gestures | earned | level | baked | served/visitors | loved");
    for (d, t) in totals.iter().enumerate() {
        println!(
            "{:>3} | {:>8.1} | {:>6.1} | {:>5.1} | {:>5.1} | {:>6.1}/{:<6.1} | {:>5.1}",
            d + 1,
            t.0 as f64 / n,
            t.1 as f64 / n,
            t.2 as f64 / n,
            t.6 as f64 / n,
            t.3 as f64 / n,
            t.4 as f64 / n,
            t.5 as f64 / n
        );
    }
    println!("unlock days (seed 0): {:?}", unlock_days.first());
}
