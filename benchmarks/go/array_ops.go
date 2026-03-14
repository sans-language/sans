package main

import "fmt"

func main() {
	// build 100k slice (0-indexed: 0..99999)
	a := make([]int, 100000)
	for i := range a {
		a[i] = i
	}

	// map: multiply each element by 2
	mapped := make([]int, len(a))
	for i, v := range a {
		mapped[i] = v * 2
	}

	// filter: keep even values
	filtered := make([]int, 0, len(mapped)/2)
	for _, v := range mapped {
		if v%2 == 0 {
			filtered = append(filtered, v)
		}
	}

	// sum
	sum := 0
	for _, v := range filtered {
		sum += v
	}
	fmt.Println(sum)
}
