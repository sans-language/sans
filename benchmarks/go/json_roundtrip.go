package main

import (
	"encoding/json"
	"fmt"
)

func main() {
	for iter := 0; iter < 100; iter++ {
		// build map with 1k keys
		m := make(map[string]int, 1000)
		for i := 0; i < 1000; i++ {
			key := fmt.Sprintf("key%d", i)
			m[key] = i
		}

		// stringify
		data, err := json.Marshal(m)
		if err != nil {
			panic(err)
		}

		// parse
		var out map[string]int
		if err := json.Unmarshal(data, &out); err != nil {
			panic(err)
		}
	}
	fmt.Println(999)
}
