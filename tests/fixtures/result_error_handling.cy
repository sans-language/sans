fn safe_divide(a Int, b Int) Result<Int> {
    if b == 0 {
        err("division by zero")
    } else {
        ok(a / b)
    }
}

fn main() Int {
    let r = safe_divide(10, 0)
    if r.is_err() {
        let msg = r.error()
        let fallback = r.unwrap_or(99)
        fallback
    } else {
        r.unwrap()
    }
}
