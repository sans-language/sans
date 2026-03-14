double(x:I) = x * 2
add(a:I b:I) = a + b

main() {
    nums = [1 2 3 4 5]
    total := 0
    for n in nums { total += n }

    doubled = nums.map(double)
    first = doubled[0]
    last = doubled[4]

    first + last + total
}
