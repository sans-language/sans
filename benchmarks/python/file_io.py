line = "hello world this is a line of text for file I/O benchmarking purposes\n"
content = line * 1000
with open("/tmp/sans_bench_file_io.txt", "w") as f:
    f.write(content)
with open("/tmp/sans_bench_file_io.txt", "r") as f:
    data = f.read()
print(len(data))
