# Sans AI Reference

Compact reference for LLM context injection. Use short aliases.

## Types
`I`=Int `F`=Float `B`=Bool `S`=String `R<T>`=Result<T>
Array<T> JsonValue HttpResponse HttpServer HttpRequest
Sender<T> Receiver<T> Mutex<T> JoinHandle

## Syntax
```
f(x:I y:S) I { body }     // function (fn optional)
f(x:I) = x*2              // expression function (no braces)
main() { 0 }              // return type defaults to I
x = 42                     // immutable (no let)
x := 0                     // mutable (no let mut)
x += 1                     // compound assign (+= -= *= /= %=)
[1 2 3]                    // array literal (commas optional)
a[0]                       // index read (= a.get(0))
a[0] = v                   // index write (= a.set(0 v))
cond ? a : b               // ternary
r!                         // unwrap (= r.unwrap())
obj.method                 // no-arg method (parens optional)
"hello {name}!"            // string interpolation
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
ld(msg)           log_debug(msg)        S -> I
li(msg)           log_info(msg)         S -> I
lw(msg)           log_warn(msg)         S -> I
le(msg)           log_error(msg)        S -> I
ll(level)         log_set_level(n)      I -> I
ok(v)                                   T -> R<T>
err(msg)                                S -> R<_>
```

## Methods
```
Array<T>:  push(v) pop len get(i) set(i v) remove(i) contains(v) map(f) filter(f)
String:    len substring(s e) trim starts_with(s) contains(s) split(d) replace(o n)
JsonValue: get(k) get_index(i) get_string get_int get_bool len type_of set(k v) push(v)
HttpResponse: status body header(n) ok
HttpServer:   accept
HttpRequest:  path method body respond(status body)
Result<T>:    is_ok is_err unwrap/! unwrap_or(d) error
Sender<T>:    send(v)
Receiver<T>:  recv
Mutex<T>:     lock unlock(v)
JoinHandle:   join
```

## Operators
`+ - * / %` arithmetic
`== != < > <= >=` comparison (works on I F S B)
`&& || !` boolean
`= := += -= *= /= %=` assignment
