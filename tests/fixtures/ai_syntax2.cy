fn double(x Int) Int { x * 2 }

main() I {
    nums = [1 2 3 4 5]
    total := 0
    for n in nums { total = total + n }
    doubled = nums.map(double)
    total + doubled.get(0)
}
