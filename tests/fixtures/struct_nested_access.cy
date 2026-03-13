struct Pair {
    a Int,
    b Int,
}

fn add_pair(x Int, y Int) Int {
    x + y
}

fn main() Int {
    let p = Pair { a: 10, b: 20 }
    add_pair(p.a, p.b)
}
