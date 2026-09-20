//! A minimal incremental-game loop: income, a multiplier, and a geometric upgrade ladder.
//!
//! Run with `cargo run --example idle_loop`.

use break_eternity::Decimal;

fn main() {
    let mut money = Decimal::from(10);
    let mut income = Decimal::from(1);
    let mut upgrades = Decimal::zero();
    let upgrade_base_cost = Decimal::from(25);
    let upgrade_ratio = Decimal::from_finite(1.15);

    for tick in 1..=200_000u32 {
        money += income;

        // How many upgrades can we afford right now?
        let affordable =
            Decimal::afford_geometric_series(money, upgrade_base_cost, upgrade_ratio, upgrades);
        if affordable.is_positive() {
            let cost = Decimal::sum_geometric_series(
                affordable,
                upgrade_base_cost,
                upgrade_ratio,
                upgrades,
            );
            money -= cost;
            upgrades += affordable;
            // Each upgrade multiplies income by 1.07.
            income *= Decimal::from_finite(1.07).pow(affordable);
        }

        if tick % 25_000 == 0 {
            println!(
                "tick {tick:>7}: money {:>14} income {:>14}/tick upgrades {}",
                money.to_precision(4),
                income.to_precision(4),
                upgrades
            );
        }
    }

    println!("\nlog10(money) = {}", money.log10().to_fixed(2));
    println!(
        "slog10(money) = {}",
        money
            .slog(None, break_eternity::TetrationMode::Analytic)
            .to_fixed(4)
    );
}
