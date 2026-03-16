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
"""multi\nline"""           // multiline string
-x                         // negation
import "mod"               // module import
struct S { x I, y I }      // struct
enum E { A, B(I) }         // enum
match v { E::A => 0, E::B(x) => x }
trait T { fn m(self) I }   // trait
impl T for S { fn m(self) I { self.x } }
spawn func(args)           // thread
let (tx rx) = channel<I>() // channel
mutex(val)                 // mutex
for x in arr { }           // iteration
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
fr(path)          file_read(path)       S -> S
fw(path body)     file_write(p b)       S S -> I
fa(path body)     file_append(p b)      S S -> I
fe(path)          file_exists(path)     S -> B
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
cors(req origin)                        HttpRequest S -> I (set CORS headers)
cors_all(req)                           HttpRequest -> I (set CORS headers wildcard)
ld(msg)           log_debug(msg)        S -> I
li(msg)           log_info(msg)         S -> I
lw(msg)           log_warn(msg)         S -> I
le(msg)           log_error(msg)        S -> I
ll(level)         log_set_level(n)      I -> I
ok(v)                                   T -> R<T>
err(msg)                                S -> R<_>

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
Array<T>:  push(v) pop len get(i) set(i v) remove(i) contains(v) map(f) filter(f) any(f) find(f) enumerate zip(b)
Map:       set(k v) get(k) has(k) len keys vals
String:    len substring(s e)/[s:e] trim starts_with(s)/sw(s) ends_with(s)/ew(s) contains(s) split(d) replace(o n)
JsonValue: get(k) get_index(i) get_string get_int get_bool len type_of set(k v) push(v)
HttpResponse: status body header(n) ok
HttpServer:   accept
HttpRequest:  path method body header(name) set_header(name val) cookie(name) respond(status body) respond(status body ct)
Result<T>:    is_ok is_err unwrap/! unwrap_or(d) error
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
