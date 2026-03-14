fn main() Int {
    let obj = json_object()
    obj.set("x", json_int(7))
    obj.set("flag", json_bool(true))

    let s = json_stringify(obj)
    let parsed = json_parse(s)
    let x = parsed.get("x").get_int()
    let flag = parsed.get("flag").get_bool()

    if flag {
        x
    } else {
        0
    }
}
