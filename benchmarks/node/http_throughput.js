const http = require('http');
const body = '{"message":"hello","n":42}';
const len = Buffer.byteLength(body);

http.createServer((req, res) => {
    res.writeHead(200, {
        'Content-Type': 'application/json',
        'Content-Length': len,
    });
    res.end(body);
}).listen(8765);
