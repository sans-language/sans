# Automatic Gzip Compression — Design Spec

## Overview

Add automatic gzip compression to `respond()`. When the client supports gzip and the response is large enough, compress transparently. Uses zlib (`-lz`). All implemented purely in Sans.

## Behavior

`respond()` auto-compresses when ALL conditions met:
1. Client sent `Accept-Encoding` containing `gzip`
2. Response body ≥ 1024 bytes
3. No `X-No-Compress: 1` response header set by user
4. Content-Type is compressible

No user code changes needed:
```sans
req.respond(200, large_body)  // auto-gzipped if client supports it
```

Opt-out:
```sans
req.set_header("X-No-Compress", "1")
req.respond(200, large_body)  // not compressed
```

## Compressible Content Types

Compress:
- `text/*` (text/html, text/plain, text/css, text/xml)
- `application/json`
- `application/javascript`
- `application/xml`
- `image/svg+xml`

Skip (already compressed or binary):
- `image/png`, `image/jpeg`, `image/gif`
- `application/octet-stream`, `application/gzip`, `application/zip`

## New Built-in: `gzip_compress(data, len)`

Returns pointer to a 16-byte struct:
```
offset 0: compressed_data_ptr (I)
offset 8: compressed_len (I)
```

### Codegen Implementation

The self-hosted codegen emits calls to zlib functions:

```llvm
declare i32 @deflateInit2_(ptr, i32, i32, i32, i32, i32, ptr, i32)
declare i32 @deflate(ptr, i32)
declare i32 @deflateEnd(ptr)
```

`gzip_compress` codegen:
1. Allocate z_stream struct (112 bytes on 64-bit)
2. Zero it with memset
3. Call `deflateInit2_(stream, 6, 8, 15+16, 8, 0, "1.2.11", 112)`
   - level=6 (default), method=8 (deflate), windowBits=31 (15+16 for gzip), memLevel=8, strategy=0
   - The version string and struct size are for ABI compat
4. Set stream->next_in = data, stream->avail_in = len
5. Allocate output buffer (deflateBound size, roughly len + 64)
6. Set stream->next_out = output, stream->avail_out = output_size
7. Call `deflate(stream, 4)` (Z_FINISH=4)
8. compressed_len = output_size - stream->avail_out
9. Call `deflateEnd(stream)`
10. Return struct [output_ptr, compressed_len]

### z_stream layout (64-bit, 112 bytes)
```
offset 0:   next_in (ptr)
offset 8:   avail_in (i32) + padding
offset 16:  total_in (i64)
offset 24:  next_out (ptr)
offset 32:  avail_out (i32) + padding
offset 40:  total_out (i64)
offset 48:  msg (ptr)
offset 56:  state (ptr)
offset 64:  zalloc (ptr)
offset 72:  zfree (ptr)
offset 80:  opaque (ptr)
offset 88:  data_type (i32) + padding
offset 96:  adler (i64)
offset 104: reserved (i64)
```

## Updated `respond()` Flow

In `sans_http_respond_build` (runtime/server.sans):

```
sans_http_respond_build(req, fd, status, body, ct):
  body_len = slen(body)
  // Check if should compress
  should = sans_should_compress(req, body_len, ct)
  if should:
    result = gzip_compress(body, body_len)
    body = load64(result)       // compressed data
    body_len = load64(result + 8)  // compressed length
    // Add Content-Encoding header
    sans_http_request_set_header(req, "Content-Encoding", "gzip")
  // ... build response with (possibly compressed) body and body_len
```

### `sans_should_compress(req, body_len, ct)` logic:

```
1. body_len < 1024 → return 0
2. Check Accept-Encoding header contains "gzip" → if not, return 0
3. Check response headers for "X-No-Compress" → if "1", return 0
4. Check ct is compressible → if not, return 0
5. return 1
```

### `sans_is_compressible_ct(ct)`:
```
ct starts with "text/" → 1
ct == "application/json" → 1
ct == "application/javascript" → 1
ct == "application/xml" → 1
ct == "image/svg+xml" → 1
otherwise → 0
```

## Linker

Add `-lz` to the self-hosted driver's link command in `compiler/main.sans`.

zlib is available by default on macOS (system library) — no brew install needed.

## Files Changed

| File | Change |
|------|--------|
| `runtime/server.sans` | Add `sans_should_compress`, `sans_is_compressible_ct`, update `sans_http_respond_build` to auto-compress |
| `compiler/codegen.sans` | Add zlib extern declarations, codegen for `gzip_compress` (deflateInit2/deflate/deflateEnd sequence) |
| `compiler/typeck.sans` | Type check `gzip_compress` (2 args → Int) |
| `compiler/ir.sans` | IR lowering for `gzip_compress` |
| `compiler/constants.sans` | New IR opcode |
| `compiler/main.sans` | Add `-lz` to linker flags |
| Docs + editors | Documentation for auto-compression behavior |

## Testing

```bash
# Start server that returns large body
# curl with gzip:
curl -H "Accept-Encoding: gzip" http://localhost:8080/ --compressed -v
# Should see Content-Encoding: gzip in response headers

# curl without gzip:
curl http://localhost:8080/ -v
# Should NOT see Content-Encoding: gzip
```

## Dependencies

- zlib (`-lz`) — available on macOS by default, no additional installation
