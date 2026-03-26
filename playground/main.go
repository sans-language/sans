package main

import (
	"encoding/json"
	"flag"
	"log"
	"net/http"
)

var db *DB

func main() {
	addr := flag.String("addr", ":8090", "listen address")
	dbPath := flag.String("db", "playground.db", "SQLite database path")
	flag.Parse()

	var err error
	db, err = NewDB(*dbPath)
	if err != nil {
		log.Fatalf("failed to open database: %v", err)
	}
	defer db.Close()

	mux := http.NewServeMux()
	mux.HandleFunc("GET /api/health", handleHealth)
	mux.HandleFunc("POST /api/run", handleRun)
	mux.HandleFunc("POST /api/share", handleShare)
	mux.HandleFunc("GET /api/snippet/{id}", handleSnippet)

	handler := corsMiddleware(mux)

	log.Printf("playground server listening on %s", *addr)
	log.Fatal(http.ListenAndServe(*addr, handler))
}

func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "https://sans.dev")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
		if r.Method == "OPTIONS" {
			w.WriteHeader(204)
			return
		}
		next.ServeHTTP(w, r)
	})
}

func handleHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}

func handleRun(w http.ResponseWriter, r *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}

func handleShare(w http.ResponseWriter, r *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}

func handleSnippet(w http.ResponseWriter, r *http.Request) {
	http.Error(w, "not implemented", http.StatusNotImplemented)
}
