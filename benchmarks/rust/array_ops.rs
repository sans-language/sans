fn main() {
    let a: Vec<i64> = (0..100000).collect();
    let b: Vec<i64> = a.iter().map(|x| x * 2).collect();
    let c: Vec<i64> = b.iter().filter(|x| *x % 2 == 0).copied().collect();
    let sum: i64 = c.iter().sum();
    println!("{}", sum);
}
