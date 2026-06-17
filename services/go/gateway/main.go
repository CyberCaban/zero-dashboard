package main

import (
	"crypto/rand"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"strings"
	"time"

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

type RequestPayload struct {
	Customer Customer `json:"customer"`
	Message  Message  `json:"message"`
}

type WebhookRequest struct {
	Payload RequestPayload `json:"payload"`
}

type ReviewPayload struct {
	SessionID  string   `json:"session_id"`
	BusinessID string   `json:"business_id"`
	Customer   Customer `json:"customer"`
	Message    Message  `json:"message"`
}

type InboundReviewMessage struct {
	Metadata Metadata      `json:"metadata"`
	Payload  ReviewPayload `json:"payload"`
}

type AuthentikJWT struct {
	TenantID         string   `json:"tenant_id"`
	Email            string   `json:"email"`
	Username         string   `json:"preferred_username"`
	Groups           []string `json:"groups"`
	AllowedLocations []string `json:"allowed_locations"`
	Sub              string   `json:"sub"`
}

type Server struct {
	nc *nats.Conn
}

func main() {
	natsURL := os.Getenv("NATS_URL")
	if natsURL == "" {
		natsURL = "nats://localhost:4222"
	}
	nc, err := nats.Connect(natsURL)
	if err != nil {
		log.Fatalf("Error connecting to NATS: %v\n", err)
		return
	}
	defer nc.Close()
	log.Printf("Connected to NATS at %s\n", natsURL)

	app := &Server{nc: nc}

	// Handle POST /v1/webhooks/telegram/{secret}
	http.HandleFunc("/v1/webhooks/telegram/", app.handleTGWebhook)
	http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		jwtHeader := r.Header.Get("X-Authentik-Jwt")
		if jwtHeader == "" {
			log.Println("No JWT token provided in health check request")
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}
		claims, err := parseAuthentikJWT(jwtHeader)
		if err != nil {
			log.Printf("Error parsing JWT token: %v\n", err)
			http.Error(w, "Invalid JWT token", http.StatusBadRequest)
			return
		}

		fmt.Printf("Parsed claims: %v\n", claims)
		if claims.TenantID == "" || claims.TenantID == "unknown_tenant" {
			log.Printf("Invalid tenant ID in JWT token: %s\n", claims.TenantID)
			http.Error(w, "Invalid tenant ID in JWT token", http.StatusBadRequest)
			return
		}

		log.Printf("Received health check request: %s %s\n", r.Method, r.URL.Path)
		w.WriteHeader(http.StatusOK)
		w.Write(fmt.Append(nil, r.Header, r.Cookies(), r.Body))
		w.Write([]byte("OK"))
	})

	port := ":8080"
	log.Printf("Starting server on port %s\n", port)
	if err := http.ListenAndServe(port, nil); err != nil {
		log.Fatalf("Error starting server: %v\n", err)
	}
}

func parseAuthentikJWT(token string) (*AuthentikJWT, error) {
	parts := strings.Split(token, ".")
	if len(parts) != 3 {
		return nil, fmt.Errorf("invalid JWT format")
	}

	payload, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return nil, err
	}

	var jwt AuthentikJWT
	if err := json.Unmarshal(payload, &jwt); err != nil {
		return nil, err
	}
	return &jwt, nil
}

func generateRandomHex(n int) string {
	buf := make([]byte, n)
	if _, err := rand.Read(buf); err != nil {
		panic(err)
	}
	return hex.EncodeToString(buf)
}

func createNATSMessage(subject string, req WebhookRequest, businessID string) (*nats.Msg, error) {
	natsMsg := nats.NewMsg(subject)
	// Headers: X-Tenant-ID, X-User-Email, X-User-Username, X-User-Groups, X-Allowed-Locations, X-Sub
	// Body: JSON with the review data and metadata
	message := InboundReviewMessage{
		Metadata: Metadata{
			Traceparent: fmt.Sprintf("00-%s-%s-01", generateRandomHex(16), generateRandomHex(16)),
			Timestamp:   time.Now().Unix(),
		},
		Payload: ReviewPayload{
			SessionID:  generateRandomHex(16),
			BusinessID: businessID,
			Customer:   req.Payload.Customer,
			Message:    req.Payload.Message,
		},
	}
	data, err := json.Marshal(message)
	if err != nil {
		return nil, err
	}
	natsMsg.Data = data
	return natsMsg, nil
}

// Handle POST /v1/webhooks/telegram/{secret}
func (s *Server) handleTGWebhook(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// /v1/webhooks/telegram/SECRET_TOKEN_CAFE_452 -> ["v1", "webhooks", "telegram", "SECRET_TOKEN_CAFE_452"]
	pathParts := strings.Split(strings.TrimPrefix(r.URL.Path, "/"), "/")
	if len(pathParts) < 3 || pathParts[0] != "v1" || pathParts[1] != "webhooks" {
		http.Error(w, "Invalid URL path", http.StatusBadRequest)
		return
	}

	platform := pathParts[2]
	secret := pathParts[3]
	// TODO: For now, we will skip secret validation to simplify testing.
	// In production, we should validate the secret token to ensure that the request is coming from Telegram.
	//
	// expectedSecret := os.Getenv("TELEGRAM_WEBHOOK_SECRET")
	// if secret != expectedSecret {
	// 	log.Printf("Invalid secret token: %s\n", secret)
	// 	http.Error(w, "Unauthorized", http.StatusUnauthorized)
	// 	return
	// }

	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	defer r.Body.Close()

	var req WebhookRequest
	if err := json.Unmarshal(body, &req); err != nil {
		http.Error(w, "Invalid JSON payload", http.StatusBadRequest)
		return
	}

	// businessID, err = mapSecretToBusinessID(secret)
	// if err != nil {
	// 	log.Printf("Error mapping secret to business ID: %v\n", err)
	// 	http.Error(w, "Invalid secret token", http.StatusUnauthorized)
	// 	return
	// }
	businessID := secret // For simplicity, we will use the secret as the business ID. In production, you should have a proper mapping of secrets to business IDs.

	subject := fmt.Sprintf("reviews.v1.inbound.%s.%s", platform, businessID)
	message, err := createNATSMessage(subject, req, businessID)
	if err != nil {
		log.Printf("Error creating NATS message: %v\n", err)
		http.Error(w, "Error creating NATS message", http.StatusInternalServerError)
		return
	}
	log.Printf("Publishing message to topic %s\n", subject)
	msg, err := s.nc.RequestMsg(message, 5*time.Second)
	if err != nil {
		log.Printf("Error publishing message to NATS: %v\n", err)
		http.Error(w, "Error publishing message to NATS", http.StatusInternalServerError)
		return
	}
	log.Printf("Received message: %s", msg.Data)
	log.Printf("Published message to topic %s\n", subject)

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	w.Write([]byte(`{"status":"success"}`))
}
