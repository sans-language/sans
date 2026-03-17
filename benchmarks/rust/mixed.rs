use std::fs;
use std::collections::HashMap;

fn main() {
    let a: Vec<i64> = (0..10000).collect();
    let b: Vec<i64> = a.iter().map(|&x| x * 2).collect();
    let c: Vec<i64> = b.iter().filter(|&&x| x % 2 == 0).cloned().collect();
    let sum: i64 = c.iter().sum();

    // Simple JSON serialization without deps
    let s = format!("{{\"sum\":{},\"count\":{}}}", sum, c.len());
    fs::write("/tmp/sans_bench_mixed.txt", &s).unwrap();
    let raw = fs::read_to_string("/tmp/sans_bench_mixed.txt").unwrap();
    // Parse sum from JSON manually
    let sum_val: i64 = raw.split("\"sum\":").nth(1).unwrap()
        .split(',').next().unwrap()
        .split('}').next().unwrap()
        .trim().parse().unwrap();
    println!("{}", sum_val);
}
