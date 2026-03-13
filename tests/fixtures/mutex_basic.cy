fn main() Int {
    let m = mutex(10)
    let v = m.lock()
    m.unlock(v + 5)
    let v2 = m.lock()
    m.unlock(v2)
    v2
}
