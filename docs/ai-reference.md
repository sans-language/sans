# Sans AI Reference

Compact reference for LLM context injection. Use short aliases.

## Types
`I`=Int `F`=Float `B`=Bool `S`=String `R<T>`=Result<T>
Array<T> Map(`M`) JsonValue HttpResponse HttpServer HttpRequest
Sender<T> Receiver<T> Mutex<T> JoinHandle
Tuple: `(I S B)` — heterogeneous fixed-size collection

## Syntax
```
f(x:I y:S) I { body }     // function (fn optional)
f(x:I) = x*2              // expression function (no braces)
f(x:I y:I=0) = x+y        // default params (trailing, literal values)
main() { 0 }              // return type defaults to I
x = 42                     // immutable (no let)
x := 0                     // mutable (no let mut)
g x = 0                    // global mutable variable
x += 1                     // compound assign (+= -= *= /= %=)
[1 2 3]                    // array literal (commas optional)
a[0]                       // index read (= a.get(0))
a[0] = v                   // index write (= a.set(0 v))
cond ? a : b               // ternary
r!                         // unwrap (= r.unwrap())
r = may_fail()?            // try: unwrap or early-return err
obj.method                 // no-arg method (parens optional)
"hello {name}!"            // string interpolation
"val is {x + 1}"          // expression interpolation
s[0:5]                     // string slice (= s.substring(0,5))
s[6:]                      // slice to end
s[:5]                      // slice from start
match x { 1 => "a", 2 => "b", _ => "c" }  // value match (int/string)
match x { n if n > 0 => n, _ => 0 }       // match guard (binding + condition)
let (a, b) = (10 20)      // tuple destructuring
"""multi\nline"""           // multiline string
-x                         // negation
import "mod"               // module import
struct S { x I, y I }      // struct
struct Pair<A B> { a A, b B }             // generic struct
Pair<I S>{ a: 1, b: "hi" }               // generic struct instantiation
enum E { A, B(I) }         // enum
match v { E::A => 0, E::B(x) => x }
trait T { fn m(self) I }   // trait
impl T for S { fn m(self) I { self.x } }
spawn func(args)           // thread
let (tx rx) = channel<I>() // channel
mutex(val)                 // mutex
for x in arr { }           // iteration
for (k v) in m.entries() { }              // for-loop destructuring
while cond { }             // loop
break                      // exit loop
continue                   // skip to next iteration
```

## Tuples
```
(1 2 3)            // literal, no commas
t.0                // access by index
f() (I S) { ... }  // tuple return type
```

## Functions (short | long)
```
p(v)              print(v)              I/F/B/S -> I
str(n)            int_to_string(n)      I -> S
stoi(s)           string_to_int(s)      S -> I
itof(n)           int_to_float(n)       I -> F
ftoi(f)           float_to_int(f)       F -> I
ftos(f)           float_to_string(f)    F -> S
abs(n)                                  I -> I
min(a b)                                I I -> I
max(a b)                                I I -> I
range(n)                                I -> Array<I> [0..n)
range(a b)                              I I -> Array<I> [a..b)
stof(s)           string_to_float(s)    S -> F
sleep(ms)                               I -> I (pause ms)
time()/now()                            -> I (unix timestamp)
random(max)/rand(max)                   I -> I [0..max)
fr(path)          file_read(path)       S -> S
fw(path body)     file_write(p b)       S S -> I
fa(path body)     file_append(p b)      S S -> I
fe(path)          file_exists(path)     S -> B
getenv(name)/genv(name)                 S -> S (read env var, "" if unset)
mkdir(path)                             S -> I (mkdir -p, 1=ok 0=err)
rmdir(path)                             S -> I (remove empty dir, 1=ok 0=err)
remove(path)/rm(path)                   S -> I (delete file, 1=ok 0=err)
listdir(path)/ls(path)                  S -> [S] (directory listing)
is_dir(path)                            S -> B (true if directory)
sh(cmd)/shell(cmd)                      S -> S (execute, capture stdout)
jo()              json_object()         -> JsonValue
ja()              json_array()          -> JsonValue
js(s)             json_string(s)        S -> JsonValue
ji(n)             json_int(n)           I -> JsonValue
jb(b)             json_bool(b)          B -> JsonValue
jn()              json_null()           -> JsonValue
jp(s)             json_parse(s)         S -> JsonValue
jfy(v)            json_stringify(v)     JsonValue -> S
hg(url)           http_get(url)         S -> HttpResponse
hp(url body ct)   http_post(u b c)      S S S -> HttpResponse
listen(port)      http_listen(port)     I -> HttpServer
hl_s(port cert key) https_listen(p c k) I S S -> HttpServer (HTTPS/TLS)
serve(port handler)                     I Fn -> I (production server, auto-threading + keep-alive + auto-gzip)
serve_tls(port cert key handler)        I S S Fn -> I (production HTTPS server)
stream_write(w data)                    I S -> I (send chunked data)
stream_end(w)                           I -> I (end chunked stream)
cors(req origin)                        HttpRequest S -> I (set CORS headers)
cors_all(req)                           HttpRequest -> I (set CORS headers wildcard)
signal_handler(signum)                  I -> I (register signal handler)
signal_check()                          -> I (1 if signal received)
spoll(fd timeout_ms)                    I I -> I (poll fd, 1=ready 0=timeout)
ws_send(ws msg)                         I S -> I (send WS text frame)
ws_recv(ws)                             I -> S (recv WS frame, "" on close)
ws_close(ws)                            I -> I (send close frame, close socket)
serve_file(req dir)                     HttpRequest S -> I (serve static file from dir)
url_decode(s)                           S -> S (URL-decode string)
path_segment(path idx)                  S I -> S (extract URL path segment by index)
ld(msg)           log_debug(msg)        S -> I
li(msg)           log_info(msg)         S -> I
lw(msg)           log_warn(msg)         S -> I
le(msg)           log_error(msg)        S -> I
ll(level)         log_set_level(n)      I -> I
ok(v)                                   T -> R<T>
err(msg)                                S -> R<_>
err(code msg)                           I S -> R<_> (error with code)

// Low-level primitives (pointers as I)
alloc(n)                                I -> I (malloc)
dealloc(p)                              I -> I (free)
ralloc(p n)                             I I -> I (realloc)
mcpy(d s n)                             I I I -> I (memcpy)
mcmp(a b n)                             I I I -> I (memcmp)
slen(p)                                 I -> I (strlen)
load8(p)                                I -> I (load byte)
store8(p v)                             I I -> I (store byte)
load16(p)                               I -> I (load 16-bit)
store16(p v)                            I I -> I (store 16-bit)
load32(p)                               I -> I (load 32-bit)
store32(p v)                            I I -> I (store 32-bit)
load64(p)                               I -> I (load 64-bit)
store64(p v)                            I I -> I (store 64-bit)
strstr(h n)                             I I -> I (find substr)
bswap16(v)                              I -> I (byte swap 16)
exit(code)                              I -> I (exit process)
system(cmd) / sys(cmd)                  S -> I (run shell cmd, return exit code)
wfd(fd msg)                             I S -> I (write to fd)
gzip_compress(data len)                 I I -> I (gzip compress, returns ptr to [ptr, len])

// Arena allocator (phase-based, stackable up to 8 deep)
arena_begin()                           -> I (push new arena)
arena_alloc(n)                          I -> I (bump alloc from arena)
arena_end()                             -> I (free all arena memory)

// SSL (advanced — prefer https_listen for most use cases)
ssl_ctx(cert key)                       S S -> I (create SSL context)
ssl_accept(ctx fd)                      I I -> I (TLS handshake)
ssl_read(ssl buf len)                   I I I -> I (read from TLS)
ssl_write(ssl buf len)                  I I I -> I (write to TLS)
ssl_close(ssl)                          I -> I (close TLS connection)

// Sockets
sock(d t p)                             I I I -> I (socket)
sbind(fd port)                          I I -> I (bind)
slisten(fd b)                           I I -> I (listen)
saccept(fd)                             I -> I (accept)
srecv(fd buf len)                       I I I -> I (recv)
ssend(fd buf len)                       I I I -> I (send)
sclose(fd)                              I -> I (close)
rbind(fd addr len)                      I I I -> I (raw bind)
rsetsockopt(fd l o v n)                 I I I I I -> I (raw setsockopt)

// Curl
cinit()                                 -> I (curl init)
csets(h opt val)                        I I S -> I (setopt str)
cseti(h opt val)                        I I I -> I (setopt long)
cperf(h)                                I -> I (perform)
cclean(h)                               I -> I (cleanup)
cinfo(h info buf)                       I I I -> I (getinfo)
curl_slist_append(sl s)                 I I -> I (append header)
curl_slist_free(sl)                     I -> I (free headers)

// Function pointers
fptr("name")                            S -> I (get fn pointer)
fcall(ptr arg)                          I I -> I (call fn ptr)
fcall2(ptr a b)                         I I I -> I (call fn ptr 2 args)
fcall3(ptr a b c)                       I I I I -> I (call fn ptr 3 args)

// Pointer access
ptr(s)                                  S/M/Array -> I (raw pointer)
char_at(s i)                            S I -> I (byte at index)

// Map operations (explicit, for when Map is stored as Int)
mget(map key)                           I S -> I (map get, 0 if missing)
mset(map key val)                       I S I -> I (map set)
mhas(map key)                           I S -> I (map has key, 1/0)

// File I/O
read_file(path)                         S -> S (read file)
write_file(path content)                S S -> I (write file)
args()                                  -> [S] (command-line args)
```

## Methods
```
Array<T>:  push(v) pop len get(i) set(i v) remove(i) contains(v) map(f) filter(f) any(f) find(f) enumerate zip(b) sort reverse join(sep) slice(s e) reduce(init f) each(f)/for_each(f) flat_map(f) sum min max flat
Map:       set(k v) get(k) has(k) len keys vals delete(k) entries
String:    len substring(s e)/[s:e] trim starts_with(s)/sw(s) ends_with(s)/ew(s) contains(s) split(d) replace(o n) upper lower index_of(s) char_at(i)/get(i) repeat(n) to_int pad_left(w ch) pad_right(w ch) bytes
Int:       to_str/to_string
JsonValue: get(k) get_index(i) get_string get_int get_bool len type_of set(k v) push(v)
HttpResponse: status body header(n) ok
HttpServer:   accept
HttpRequest:  path method body header(name) set_header(name val) query(name) path_only content_length cookie(name) form(name) respond(status body) respond(status body ct) respond_json(status body) respond_stream(status) is_ws_upgrade upgrade_ws
              // respond auto-gzips when: body>=1024B + Accept-Encoding:gzip + compressible ct; opt-out: set_header("X-No-Compress" "1")
Result<T>:    is_ok is_err unwrap/! unwrap_or(d) error code
Sender<T>:    send(v)
Receiver<T>:  recv
Mutex<T>:     lock unlock(v)
JoinHandle:   join
```

## Lambdas & Closures
```
|x:I| I { x + 10 }              // lambda expression
f = |x:I| I { x * 2 }           // assign to variable
f(5)                             // call: 10
a.map(|x:I| I { x * 2 })        // pass to map
offset = 10
g = |x:I| I { x + offset }      // implicit capture
g(5)                             // 15
```

## Map
```
m = M()                    // create empty map
m.set("key" 42)            // set key-value
m.get("key")               // get value (0 if missing)
m.has("key")               // B — key exists?
m.len()                    // I — entry count
m.keys()                   // [S] — all keys
m.vals()                   // [I] — all values
```

## Iterator Chains
```
a.map(|x:I| I { x * 2 }).filter(|x:I| B { x > 3 })  // chained, auto-materialized
a.any(|x:I| B { x > 3 })       // B — true if any match
a.find(|x:I| B { x > 3 })      // I — first match or 0
a.enumerate()                    // [(I I)] — index-value tuples
a.zip(b)                         // [(I I)] — paired tuples
```

## Operators
`+ - * / %` arithmetic
`== != < > <= >=` comparison (works on I F S B)
`&& || !` boolean
`= := += -= *= /= %=` assignment
`?` try (on R<T>: unwrap or early-return err)

## Builtin Names (user-defined functions take precedence)
User functions override builtins of the same name. Builtin names: `p serve serve_file serve_tls listen alloc load8/16/32/64 store8/16/32/64 mcpy slen wfd ok err exit sys str stoi itof ftoi ftos fr fw fa fe jp jfy jo ja map M sock saccept srecv ssend sclose args signal_handler signal_check` and all others listed above.

## All Aliases (short | medium | long)
fread/fr/file_read  fwrite/fw/file_write  fappend/fa/file_append  fexists/fe/file_exists
itos/str/int_to_string  jparse/jp/json_parse  jobj/jo/json_object  jarr/ja/json_array
jstr/js/json_string  jstringify/jfy/json_stringify  hget/hg/http_get  hpost/hp/http_post
hl/listen/http_listen  HS=HttpServer  HR=HttpRequest
ab/arena_begin  aa/arena_alloc  ae/arena_end  gz/gzip_compress
ca/cors_all  ud/url_decode  ps/path_segment  sigh/signal_handler  sigc/signal_check
idx/index_of  pl/pad_left  pr/pad_right  ti/to_int  fm/flat_map
gidx/get_index  gs/get_string  geti/get_int  gb/get_bool  typeof/type_of
cl/content_length  rj/respond_json
getenv/genv  remove/rm  listdir/ls  sh/shell
