let total = 0;
for (let i = 0; i < 100000; i++) {
    const s = "hello" + "world" + "hello" + "world" + "hello";
    total += s.length;
}
console.log(total);
