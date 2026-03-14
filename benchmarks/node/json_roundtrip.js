let result = 0;
for (let i = 0; i < 100; i++) {
    const obj = {};
    for (let k = 0; k < 1000; k++) obj[String(k)] = k;
    const s = JSON.stringify(obj);
    const parsed = JSON.parse(s);
    result = parsed["999"];
}
console.log(result);
