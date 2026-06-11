FROM golang:1.26.1-alpine AS builder
RUN apk add --no-cache git

WORKDIR /app

# COPY api/ ./api/

COPY services/go/gateway/go.mod services/go/gateway/go.sum ./services/go/gateway/

WORKDIR /app/services/go/gateway
RUN go mod download

COPY services/go/gateway/ .

RUN --mount=type=cache,target=/root/.cache/go-build \
    go build -ldflags="-s -w" -o /bin/app ./main.go

FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=builder /bin/app /app
CMD ["/app"]