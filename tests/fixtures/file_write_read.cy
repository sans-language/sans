fn main() Int {
    let ok = file_write("/tmp/cyflym_test_write.txt", "hello world")
    let content = file_read("/tmp/cyflym_test_write.txt")
    content.len()
}
