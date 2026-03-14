fn main() Int {
    let obj = json_object()
    obj.set("name", json_string("cyflym"))
    obj.set("version", json_int(1))

    let tags = json_array()
    tags.push(json_string("fast"))
    tags.push(json_string("safe"))
    obj.set("tags", tags)

    let s = json_stringify(obj)
    s.len()
}
