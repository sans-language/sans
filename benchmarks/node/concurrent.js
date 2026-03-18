const { Worker, isMainThread, parentPort, workerData } = require('worker_threads');

if (isMainThread) {
    let done = 0;
    let total = BigInt(0);
    for (let i = 0; i < 4; i++) {
        const w = new Worker(__filename, { workerData: i });
        w.on('message', (v) => {
            total += BigInt(v);
            done++;
            if (done === 4) console.log(total.toString());
        });
    }
} else {
    let sum = BigInt(0);
    for (let i = BigInt(1); i <= BigInt(1000000); i++) sum += i;
    parentPort.postMessage(sum.toString());
}
