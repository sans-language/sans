# WebSocket Support (Text + Ping/Pong) — Design Spec

## Overview

Add WebSocket support to the Sans HTTP server: upgrade handshake, text message send/receive, ping/pong keepalive, and close. Integrates with existing `serve()` handler pattern. Requires SHA-1 and Base64 implementations in pure Sans.

## 1. API

```sans
handle(req:I) I {
  req.is_ws_upgrade() ? {
    ws = req.upgrade_ws()
    msg := ws.recv()
    while msg != "" {
      ws.send("echo: " + msg)
      msg = ws.recv()
    }
    ws.close()
  } : {
    req.respond(200, "Not a WebSocket")
  }
}

main() I {
  serve(8080, fptr("handle"))
}
```

## 2. Upgrade Handshake (RFC 6455 Section 4)

### `req.is_ws_upgrade()` → Int (1/0)

Returns 1 if request has both:
- `Upgrade: websocket` header (case-insensitive)
- `Connection: Upgrade` header (or contains "upgrade")

### `req.upgrade_ws()` → Int (WebSocket struct pointer)

1. Read `Sec-WebSocket-Key` header
2. Concatenate with magic GUID `258EAFA5-E914-47DA-95CA-C5AB0DC85B11`
3. SHA-1 hash the concatenated string
4. Base64 encode the 20-byte hash
5. Send 101 response:
```
HTTP/1.1 101 Switching Protocols\r\n
Upgrade: websocket\r\n
Connection: Upgrade\r\n
Sec-WebSocket-Accept: {base64_sha1}\r\n
\r\n
```
6. Return WebSocket struct

## 3. WebSocket Struct (32 bytes)

```
offset 0:  fd (I)   — socket fd
offset 8:  ssl (I)  — SSL pointer (0 for plain WS, non-zero for WSS)
offset 16: open (I) — 1 if connection open, 0 if closed
offset 24: req (I)  — back-pointer to original request
```

## 4. Frame Format (RFC 6455 Section 5)

```
Byte 0: [FIN:1][RSV:3][opcode:4]
Byte 1: [MASK:1][payload_len:7]
If payload_len == 126: bytes 2-3 = 16-bit length (network order)
If payload_len == 127: bytes 2-9 = 64-bit length (network order)
If MASK: next 4 bytes = masking key
Then: payload data
```

### Opcodes
- 0x1 = text frame
- 0x8 = close
- 0x9 = ping
- 0xA = pong

### Rules
- Server→client: NOT masked (MASK=0)
- Client→server: MUST be masked (MASK=1, 4-byte XOR key)
- FIN=1 for all frames (no fragmentation support)

## 5. Methods

### `ws.send(msg:S)` → Int

Send text frame:
1. Byte 0: 0x81 (FIN=1, opcode=1)
2. Byte 1: payload length (no mask)
   - If len ≤ 125: 1 byte
   - If len ≤ 65535: byte 1 = 126, then 2 bytes big-endian length
   - If len > 65535: byte 1 = 127, then 8 bytes big-endian length
3. Payload bytes
4. Send via ssend/ssl_write

### `ws.recv()` → String

Receive next frame:
1. Read 2 header bytes
2. Extract: FIN (bit 7 of byte 0), opcode (bits 0-3 of byte 0), MASK (bit 7 of byte 1), payload_len (bits 0-6 of byte 1)
3. If payload_len == 126: read 2 more bytes → 16-bit big-endian length
4. If payload_len == 127: read 8 more bytes → 64-bit big-endian length
5. If MASK=1: read 4 mask bytes
6. Read payload_len bytes of payload
7. If MASK=1: XOR each byte with mask[i % 4]
8. Dispatch on opcode:
   - 0x1 (text): return payload as string
   - 0x8 (close): send close frame back, set open=0, return ""
   - 0x9 (ping): send pong frame with same payload, loop to step 1
   - 0xA (pong): ignore, loop to step 1
   - Other: ignore, loop to step 1

### `ws.close()` → Int

1. If already closed (open=0): return 0
2. Send close frame: bytes [0x88, 0x00] (FIN=1, opcode=8, len=0)
3. Set open=0
4. Close socket (sclose/ssl_close)

## 6. SHA-1 Implementation (Pure Sans)

Standard SHA-1 (FIPS 180-4):

### Constants
- h0=0x67452301, h1=0xEFCDAB89, h2=0x98BADCFE, h3=0x10325476, h4=0xC3D2E1F0

### Algorithm
1. Pre-process: pad message to multiple of 64 bytes (append 0x80, zeros, 64-bit length big-endian)
2. Process each 64-byte block:
   a. Expand 16 words to 80 words: w[i] = rotl(w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16], 1)
   b. Initialize a,b,c,d,e from h0-h4
   c. 80 rounds: temp = rotl(a,5) + f(b,c,d) + e + k + w[i]; e=d; d=c; c=rotl(b,30); b=a; a=temp
   d. h0+=a, h1+=b, h2+=c, h3+=d, h4+=e
3. Output: h0||h1||h2||h3||h4 (20 bytes big-endian)

### Bit rotation
`rotl32(x, n)` = `((x << n) | (x >> (32 - n))) & 0xFFFFFFFF`

Need 32-bit masking since Sans uses 64-bit integers.

### f(b,c,d) by round
- 0-19: (b & c) | (~b & d), k=0x5A827999
- 20-39: b ^ c ^ d, k=0x6ED9EBA1
- 40-59: (b & c) | (b & d) | (c & d), k=0x8F1BBCDC
- 60-79: b ^ c ^ d, k=0xCA62C1D6

## 7. Base64 Encode (Pure Sans)

Encoding table: `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/`

Process 3 input bytes → 4 output chars:
1. Combine 3 bytes into 24-bit value
2. Split into 4 6-bit indices
3. Map each to base64 char
4. Pad with `=` if input not multiple of 3

## 8. Helper: Socket Read Exact

`sans_ws_read_exact(fd, ssl, buf, n)` — read exactly n bytes, handling partial reads. Loop `recv` until n bytes accumulated.

## 9. Files Changed

| File | Change |
|------|--------|
| `runtime/server.sans` | Add: `sans_ws_is_upgrade`, `sans_ws_upgrade`, `sans_ws_send`, `sans_ws_recv`, `sans_ws_close`, `sans_sha1`, `sans_sha1_block`, `sans_sha1_pad`, `sans_base64_encode`, `sans_ws_read_exact`, `sans_ws_send_frame`, `sans_ws_send_pong`, frame parsing helpers |
| `compiler/typeck.sans` | Methods on HttpRequest: `is_ws_upgrade` (0 args → Int), `upgrade_ws` (0 args → Int). Functions/methods on WS: `ws_send`/`send`, `ws_recv`/`recv`, `ws_close`/`close` |
| `compiler/ir.sans` | IR lowering for all WS functions |
| `compiler/codegen.sans` | Extern declarations |
| `compiler/constants.sans` | New IR opcodes |
| `examples/websocket_server.sans` | Echo server example |
| Docs + editors | All documentation |

## 10. Testing

### Echo Server
```sans
handle(req:I) I {
  req.is_ws_upgrade() ? {
    ws = req.upgrade_ws()
    p("WS connected")
    msg := ws.recv()
    while msg != "" {
      p("Got: " + msg)
      ws.send("echo: " + msg)
      msg = ws.recv()
    }
    p("WS closed")
    ws.close()
  } : {
    req.respond(200, "WebSocket server - connect to ws://localhost:8080")
  }
}

main() I {
  p("ws://localhost:8080")
  serve(8080, fptr("handle"))
}
```

Test with browser console:
```js
ws = new WebSocket("ws://localhost:8080")
ws.onmessage = e => console.log(e.data)
ws.send("hello")
// Should print: echo: hello
```

Or with `websocat ws://localhost:8080`.

## 11. Dependencies

None — SHA-1 and Base64 implemented in pure Sans. Uses existing socket/SSL primitives.
