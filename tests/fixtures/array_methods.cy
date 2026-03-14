fn bool_to_int(b Bool) Int {
    if b { 1 } else { 0 }
}

fn main() Int {
    let a = array<Int>()
    a.push(10)
    a.push(20)
    a.push(30)

    let has_20 = bool_to_int(a.contains(20))
    let has_99 = bool_to_int(a.contains(99))

    let popped = a.pop()
    let new_len = a.len()

    // has_20=1, has_99=0, popped=30, new_len=2
    // result = 1 + 0 + 30 + 2 = 33
    has_20 + has_99 + popped + new_len
}
