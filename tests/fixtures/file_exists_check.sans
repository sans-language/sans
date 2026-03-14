fn main() Int {
    file_write("/tmp/cyflym_test_exists.txt", "test")
    let a = file_exists("/tmp/cyflym_test_exists.txt")
    let b = file_exists("/tmp/cyflym_nonexistent_file_xyz.txt")
    if a {
        if b {
            11
        } else {
            1
        }
    } else {
        if b {
            10
        } else {
            0
        }
    }
}
