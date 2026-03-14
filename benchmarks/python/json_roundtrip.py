import json
result = 0
for _ in range(100):
    obj = {str(k): k for k in range(1000)}
    s = json.dumps(obj)
    parsed = json.loads(s)
    result = parsed["999"]
print(result)
