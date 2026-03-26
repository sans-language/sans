package main

import (
	"crypto/rand"
	"database/sql"
	"math/big"
	"time"

	_ "github.com/mattn/go-sqlite3"
)

const base62 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"

type DB struct {
	conn *sql.DB
}

func NewDB(path string) (*DB, error) {
	conn, err := sql.Open("sqlite3", path+"?_journal_mode=WAL")
	if err != nil {
		return nil, err
	}
	if _, err := conn.Exec(`
		CREATE TABLE IF NOT EXISTS snippets (
			id TEXT PRIMARY KEY,
			code TEXT NOT NULL,
			created_at INTEGER NOT NULL
		);
		CREATE TABLE IF NOT EXISTS compile_logs (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			code_length INTEGER NOT NULL,
			compile_success BOOLEAN NOT NULL,
			created_at INTEGER NOT NULL
		);
	`); err != nil {
		return nil, err
	}
	return &DB{conn: conn}, nil
}

func (db *DB) Close() error {
	return db.conn.Close()
}

func generateID() (string, error) {
	b := make([]byte, 8)
	for i := range b {
		n, err := rand.Int(rand.Reader, big.NewInt(62))
		if err != nil {
			return "", err
		}
		b[i] = base62[n.Int64()]
	}
	return string(b), nil
}

func (db *DB) SaveSnippet(code string) (string, error) {
	id, err := generateID()
	if err != nil {
		return "", err
	}
	_, err = db.conn.Exec(
		"INSERT INTO snippets (id, code, created_at) VALUES (?, ?, ?)",
		id, code, time.Now().Unix(),
	)
	if err != nil {
		return "", err
	}
	return id, nil
}

func (db *DB) GetSnippet(id string) (string, error) {
	var code string
	err := db.conn.QueryRow("SELECT code FROM snippets WHERE id = ?", id).Scan(&code)
	if err != nil {
		return "", err
	}
	return code, nil
}

func (db *DB) LogCompile(codeLength int, success bool) {
	db.conn.Exec(
		"INSERT INTO compile_logs (code_length, compile_success, created_at) VALUES (?, ?, ?)",
		codeLength, success, time.Now().Unix(),
	)
}
