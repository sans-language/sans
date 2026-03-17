package main

import (
	"fmt"
	"sync"
)

func worker(ch chan<- int64, wg *sync.WaitGroup) {
	defer wg.Done()
	var sum int64
	for i := int64(1); i <= 1000000; i++ {
		sum += i
	}
	ch <- sum
}

func main() {
	results := make(chan int64, 4)
	var wg sync.WaitGroup
	for i := 0; i < 4; i++ {
		wg.Add(1)
		go worker(results, &wg)
	}
	wg.Wait()
	close(results)
	var total int64
	for v := range results {
		total += v
	}
	fmt.Println(total)
}
