fn adder(a Int, b Int) Int {
    a + b
}

fn main() Int {
    let (tx, rx) = channel<Int>()
    tx.send(10)
    let h = spawn adder(3, 4)
    let val = rx.recv()
    h.join()
    val
}
