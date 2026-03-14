fn main() {
    let mut total: usize = 0;
    for _ in 0..100000 {
        let s = String::from("hello") + "world" + "hello" + "world" + "hello";
        total += s.len();
    }
    println!("{}", total);
}
