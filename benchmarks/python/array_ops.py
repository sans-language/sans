a = list(range(100000))
b = list(map(lambda x: x * 2, a))
c = list(filter(lambda x: x % 2 == 0, b))
print(sum(c))
