package sample

import (
	"database/sql"
	"net/http"
)

func load() {
	resp, _ := http.Get("https://api.axhub.dev/v1/posts")
	conn, _ := sql.Open("postgres", "dsn")
	url := "https://backend.example.com/data"
	_ = resp
	_ = conn
	_ = url
}
