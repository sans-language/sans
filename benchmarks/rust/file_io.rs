use std::fs;

fn main() {
    let line = "hello world this is a line of text for file I/O benchmarking purposes\n";
    let content = line.repeat(1000);
    fs::write("/tmp/sans_bench_file_io.txt", &content).unwrap();
    let data = fs::read_to_string("/tmp/sans_bench_file_io.txt").unwrap();
    println!("{}", data.len());
}
