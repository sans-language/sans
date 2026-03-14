fn main() Int {
    let a = log_set_level(2)
    let b = log_debug("should not print")
    let c = log_info("should not print")
    let d = log_warn("this prints")
    let e = log_error("this prints too")
    a + b + c + d + e
}
