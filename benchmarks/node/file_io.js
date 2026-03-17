const fs = require('fs');
const line = "hello world this is a line of text for file I/O benchmarking purposes\n";
const content = line.repeat(1000);
fs.writeFileSync("/tmp/sans_bench_file_io.txt", content);
const data = fs.readFileSync("/tmp/sans_bench_file_io.txt", 'utf8');
console.log(data.length);
