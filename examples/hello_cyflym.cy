fn main() Int {
    log_info("Hello from Cyflym!")

    // JSON
    let data = json_object()
    data.set("language", json_string("cyflym"))
    data.set("version", json_int(1))
    data.set("features", json_array())
    data.get("features").push(json_string("fast"))
    data.get("features").push(json_string("safe"))
    data.get("features").push(json_string("concurrent"))

    let json_out = json_stringify(data)
    print(json_out)

    // String operations
    let name = "Cyflym"
    let msg = "Welcome to " + name + "!"
    print(msg)

    // Math
    let mut sum = 0
    let mut i = 1
    while i <= 10 {
        sum = sum + i
        i = i + 1
    }
    print(int_to_string(sum))

    log_info("Done!")
    0
}
