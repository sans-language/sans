const fs = require('fs');

const a = Array.from({length: 10000}, (_, i) => i);
const b = a.map(x => x * 2);
const c = b.filter(x => x % 2 === 0);
const total = c.reduce((s, x) => s + x, 0);

const obj = {sum: total, count: c.length};
const s = JSON.stringify(obj);
fs.writeFileSync("/tmp/sans_bench_mixed.txt", s);
const data = fs.readFileSync("/tmp/sans_bench_mixed.txt", 'utf8');
const parsed = JSON.parse(data);
console.log(parsed.sum);
