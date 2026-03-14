fn main() Int {
    let s = "{\"a\":10,\"b\":20,\"nested\":{\"c\":12}}"
    let v = json_parse(s)
    let a = v.get("a").get_int()
    let b = v.get("b").get_int()
    let c = v.get("nested").get("c").get_int()
    a + b + c
}
