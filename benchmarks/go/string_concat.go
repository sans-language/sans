package main

import "fmt"

func main() {
	total := 0
	for i := 0; i < 100000; i++ {
		s := "hello" + "world" + "hello" + "world" + "hello"
		total += len(s)
	}
	fmt.Println(total)
}
