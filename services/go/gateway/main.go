package main

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"strings"

	"github.com/nats-io/nats.go"
)

type Metadata struct {
	Traceparent string `json:"traceparent"`
	Timestamp   int64  `json:"timestamp"`
}

type Customer struct {
	PlatformUserID string `json:"platform_user_id"`
	Phone          string `json:"phone"`
	Name           string `json:"name"`
}

type Message struct {
	MessageID string `json:"message_id"`
	Text      string `json:"text"`
	RawRating int    `json:"raw_rating"`
}

type Payload struct {
	SessionID  string   `json:"session_id"`
	BusinessID string   `json:"business_id"`
	Customer   Customer `json:"customer"`
	Message    Message  `json:"message"`
}

type ReviewRequest struct {
	Metadata Metadata `json:"metadata"`
	Payload  Payload  `json:"payload"`
}

type Server struct {
	nc *nats.Conn
}

func main() {
	natsURL := os.Getenv("NATS_URL")
	if natsURL == "" {
		natsURL = "nats://127.0.0.1:4222"
	}
	nc, err := nats.Connect(natsURL)
	if err != nil {
		log.Fatalf("Error connecting to NATS: %v\n", err)
		return
	}
	defer nc.Close()
	log.Printf("Connected to NATS at %s\n", natsURL)

	app := &Server{nc: nc}

	// Handle POST /review/{platform}
	http.HandleFunc("/review/", app.handleReview)

	port := ":8080"
	log.Printf("Starting server on port %s\n", port)
	if err := http.ListenAndServe(port, nil); err != nil {
		log.Fatalf("Error starting server: %v\n", err)
	}
}

func (s *Server) handleReview(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// /review/{platform} -> ["review", "{platform}"]
	pathParts := strings.Split(strings.TrimPrefix(r.URL.Path, "/"), "/")
	if len(pathParts) != 2 || pathParts[1] == "" {
		http.Error(w, "No platform specified", http.StatusBadRequest)
		return
	}

	platform := pathParts[1]

	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	defer r.Body.Close()

	var req ReviewRequest
	if err := json.Unmarshal(body, &req); err != nil {
		http.Error(w, "Invalid JSON payload", http.StatusBadRequest)
		return
	}

	businessID := req.Payload.BusinessID
	if businessID == "" {
		http.Error(w, "Business ID is required", http.StatusBadRequest)
		return
	}

	topic := fmt.Sprintf("reviews.v1.inbound.%s.%s", platform, businessID)
	err = s.nc.Publish(topic, body)
	if err != nil {
		log.Printf("Error publishing message to NATS: %v\n", err)
		http.Error(w, "Error publishing message to NATS", http.StatusInternalServerError)
		return
	}
	log.Printf("Published message to topic %s\n", topic)

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	w.Write([]byte(`{"status":"success"}`))
}
