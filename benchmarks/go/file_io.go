package main

import (
	"fmt"
	"os"
	"strings"
)

func main() {
	line := "hello world this is a line of text for file I/O benchmarking purposes\n"
	content := strings.Repeat(line, 1000)
	os.WriteFile("/tmp/sans_bench_file_io.txt", []byte(content), 0644)
	data, _ := os.ReadFile("/tmp/sans_bench_file_io.txt")
	fmt.Println(len(data))
}
