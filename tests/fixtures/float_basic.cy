fn main() Int {
    let pi = 3.14
    let r = 2.0
    let area = pi * r * r
    print(area)

    let a = 10.5
    let b = 3.2
    let sum = a + b
    let diff = a - b
    let prod = a * b
    let quot = a / b
    print(sum)

    // Comparisons
    if a > b {
        print("a > b")
    }

    // Conversions
    let n = float_to_int(area)
    let f = int_to_float(42)
    let s = float_to_string(pi)
    print(s)
    print(int_to_string(n))

    n
}
