package main

import (
	"encoding/json"
	"fmt"
	"os"
)

func main() {
	a := make([]int, 10000)
	for i := range a { a[i] = i }
	b := make([]int, len(a))
	for i, v := range a { b[i] = v * 2 }
	var c []int
	for _, v := range b {
		if v%2 == 0 { c = append(c, v) }
	}
	sum := 0
	for _, v := range c { sum += v }

	obj := map[string]int{"sum": sum, "count": len(c)}
	data, _ := json.Marshal(obj)
	os.WriteFile("/tmp/sans_bench_mixed.txt", data, 0644)
	raw, _ := os.ReadFile("/tmp/sans_bench_mixed.txt")
	var parsed map[string]int
	json.Unmarshal(raw, &parsed)
	fmt.Println(parsed["sum"])
}
