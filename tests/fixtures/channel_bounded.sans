fn main() Int {
    let (tx, rx) = channel<Int>(2)
    tx.send(10)
    tx.send(20)
    let a = rx.recv()
    let b = rx.recv()
    a + b
}
