const a = Array.from({length: 100000}, (_, i) => i);
const b = a.map(x => x * 2);
const c = b.filter(x => x % 2 === 0);
console.log(c.reduce((s, x) => s + x, 0));
