//! Every way to turn a `Decimal` into text, side by side.
//!
//! Run with `cargo run --example formatting`.

use break_eternity::Decimal;

fn main() {
    let values = [
        "0",
        "-5",
        "1234.5678",
        "0.00123",
        "1e21",
        "1.5e100",
        "-2.5e-300",
        "1e1000",
        "ee100.5",
        "(e^10)33.3",
        "Infinity",
    ];
    println!(
        "{:>12} | {:>24} | {:>14} | {:>14} | {:>14} | {:>12}",
        "input", "Display", "to_fixed(2)", "to_precision(3)", "to_exponential(2)", "{:.1e}"
    );
    for s in values {
        let d: Decimal = s.parse().expect("valid literal");
        println!(
            "{s:>12} | {:>24} | {:>14} | {:>14} | {:>14} | {:>12}",
            d.to_string(),
            d.to_fixed(2),
            d.to_precision(3),
            d.to_exponential(2),
            format!("{d:.1e}")
        );
    }

    println!("\nAccepted input notations for the same value (10^10^100):");
    for s in [
        "ee100",
        "1e1e100",
        "(e^2)100",
        "2pt100",
        "2 PT (100)",
        "100f2",
        "10^^2;100",
    ] {
        println!("  {s:>12} -> {}", s.parse::<Decimal>().unwrap());
    }
}
