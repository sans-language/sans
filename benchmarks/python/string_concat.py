total = 0
for _ in range(100000):
    s = "hello" + "world" + "hello" + "world" + "hello"
    total += len(s)
print(total)
