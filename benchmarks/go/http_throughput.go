package main

import (
	"fmt"
	"net/http"
)

func main() {
	body := `{"message":"hello","n":42}`
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprint(w, body)
	})
	http.ListenAndServe(":8765", nil)
}
