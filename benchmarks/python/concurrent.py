import threading

def worker(results, idx):
    s = sum(range(1, 1000001))
    results[idx] = s

results = [0] * 4
threads = [threading.Thread(target=worker, args=(results, i)) for i in range(4)]
for t in threads:
    t.start()
for t in threads:
    t.join()
print(sum(results))
