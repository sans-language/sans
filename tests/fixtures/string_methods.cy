fn bool_to_int(b Bool) Int {
    if b { 1 } else { 0 }
}

fn main() Int {
    let s = "  hello world  "
    let trimmed = s.trim()
    let tlen = trimmed.len()

    let greeting = "hello world"
    let sw = bool_to_int(greeting.starts_with("hello"))
    let cw = bool_to_int(greeting.contains("world"))
    let cf = bool_to_int(greeting.contains("foo"))

    let csv = "a,b,c,d"
    let parts = csv.split(",")
    let num_parts = parts.len()

    // tlen=11, sw=1, cw=1, cf=0, num_parts=4
    // result = 11 + 1 + 1 + 0 + 4 = 17
    tlen + sw + cw + cf + num_parts
}
