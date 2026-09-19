//! Criterion benchmarks for the hot arithmetic paths at layers 0 through 3.

use break_eternity::{Decimal, TetrationMode};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn samples() -> Vec<(&'static str, Decimal)> {
    vec![
        ("layer0", Decimal::from_finite(123_456.789)),
        ("layer1", "1.5e1000".parse().unwrap()),
        ("layer2", "ee100.5".parse().unwrap()),
        ("layer3", "eee20".parse().unwrap()),
    ]
}

fn bench_arithmetic(c: &mut Criterion) {
    let mult = Decimal::from_finite(1.0001);
    for (name, x) in samples() {
        let y = x * Decimal::from_finite(3.7);
        c.bench_function(&format!("add/{name}"), |b| {
            b.iter(|| black_box(x) + black_box(y))
        });
        c.bench_function(&format!("mul/{name}"), |b| {
            b.iter(|| black_box(x) * black_box(mult))
        });
        c.bench_function(&format!("div/{name}"), |b| {
            b.iter(|| black_box(x) / black_box(y))
        });
        c.bench_function(&format!("cmp/{name}"), |b| {
            b.iter(|| black_box(x).cmp(&black_box(y)))
        });
    }
}

fn bench_transcendental(c: &mut Criterion) {
    let exp = Decimal::from_finite(1.15);
    for (name, x) in samples() {
        c.bench_function(&format!("pow/{name}"), |b| {
            b.iter(|| black_box(x).pow(black_box(exp)))
        });
        c.bench_function(&format!("log10/{name}"), |b| {
            b.iter(|| black_box(x).log10())
        });
        c.bench_function(&format!("sqrt/{name}"), |b| b.iter(|| black_box(x).sqrt()));
    }
    c.bench_function("tetrate/10^^2.5", |b| {
        b.iter(|| Decimal::ten().tetrate(Some(black_box(2.5)), None, TetrationMode::Analytic))
    });
    let tower: Decimal = "ee1000".parse().unwrap();
    c.bench_function("slog/ee1000", |b| {
        b.iter(|| black_box(tower).slog(None, TetrationMode::Analytic))
    });
}

fn bench_strings(c: &mut Criterion) {
    for (name, x) in samples() {
        let s = x.to_string();
        c.bench_function(&format!("display/{name}"), |b| {
            b.iter(|| black_box(x).to_string())
        });
        c.bench_function(&format!("parse/{name}"), |b| {
            b.iter(|| black_box(s.as_str()).parse::<Decimal>().unwrap())
        });
        c.bench_function(&format!("to_fixed/{name}"), |b| {
            b.iter(|| black_box(x).to_fixed(2))
        });
    }
}

fn bench_game_loop(c: &mut Criterion) {
    c.bench_function("idle_tick_x1000", |b| {
        b.iter(|| {
            let mut money = Decimal::from(10);
            let per_tick: Decimal = "1.5e10".parse().unwrap();
            let mult = Decimal::from_finite(1.0001);
            for _ in 0..1000 {
                money = (money + per_tick) * mult;
            }
            money
        })
    });
}

criterion_group!(
    benches,
    bench_arithmetic,
    bench_transcendental,
    bench_strings,
    bench_game_loop
);
criterion_main!(benches);
