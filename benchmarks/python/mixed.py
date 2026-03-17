import json

a = list(range(10000))
b = [x * 2 for x in a]
c = [x for x in b if x % 2 == 0]
total = sum(c)

obj = {"sum": total, "count": len(c)}
s = json.dumps(obj)
with open("/tmp/sans_bench_mixed.txt", "w") as f:
    f.write(s)
with open("/tmp/sans_bench_mixed.txt", "r") as f:
    data = f.read()
parsed = json.loads(data)
print(parsed["sum"])
