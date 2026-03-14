fn main() Int {
    let r = http_get("http://invalid.invalid.invalid")
    let s = r.status()
    let b = r.body()
    if s == 0 {
        if r.ok() {
            0
        } else {
            1
        }
    } else {
        0
    }
}
