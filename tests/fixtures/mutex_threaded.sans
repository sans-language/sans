fn worker(val Int) Int {
    val + 1
}

fn main() Int {
    let m = mutex(0)
    let v1 = m.lock()
    let h1 = spawn worker(v1)
    m.unlock(v1)
    h1.join()
    let v2 = m.lock()
    m.unlock(v2 + 1)
    let v3 = m.lock()
    m.unlock(v3)
    v3
}
