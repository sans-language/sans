use std::thread;
use std::sync::mpsc;

fn main() {
    let (tx, rx) = mpsc::channel::<i64>();
    let mut handles = Vec::new();
    for _ in 0..4 {
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            let sum: i64 = (1..=1_000_000).sum();
            tx.send(sum).unwrap();
        }));
    }
    drop(tx);
    let total: i64 = rx.iter().sum();
    for h in handles { h.join().unwrap(); }
    println!("{}", total);
}
