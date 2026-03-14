fn divide(a Int, b Int) Result<Int> {
    if b == 0 {
        err("division by zero")
    } else {
        ok(a / b)
    }
}

fn main() Int {
    let r1 = divide(10, 2)
    let r2 = divide(20, 4)
    let v1 = r1.unwrap()
    let v2 = r2.unwrap()
    v1 + v2
}
