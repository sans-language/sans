divide(a:I b:I) R<I> = b==0 ? err("div/0") : ok(a/b)

main() {
    r = divide(10 2)
    v = r!
    v
}
