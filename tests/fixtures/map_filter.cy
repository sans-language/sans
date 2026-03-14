fn double(x Int) Int {
    x * 2
}

fn is_even(x Int) Bool {
    let rem = x - (x / 2) * 2
    if rem == 0 { true } else { false }
}

fn main() Int {
    let a = array<Int>()
    a.push(1)
    a.push(2)
    a.push(3)
    a.push(4)
    a.push(5)

    // Map: [1,2,3,4,5] -> [2,4,6,8,10]
    let doubled = a.map(double)
    let d_len = doubled.len()

    // Filter: [1,2,3,4,5] -> [2,4]
    let evens = a.filter(is_even)
    let e_len = evens.len()

    // Get values
    let d0 = doubled.get(0)
    let d4 = doubled.get(4)
    let e0 = evens.get(0)

    // d_len=5, e_len=2, d0=2, d4=10, e0=2
    // result = 5 + 2 + 2 + 10 + 2 = 21
    d_len + e_len + d0 + d4 + e0
}
