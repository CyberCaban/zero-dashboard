### Архитектура взаимодействия:

1.  **Пользователь кликает по QR-коду или ссылке:**
    *   Ссылка ведет на фронтенд-сервис: `https://review.zero-dashboard.local/r/cafe_452?session=...` (или на защищенный эндпоинт API-шлюза, который отдает страницу).
2.  **Браузер загружает страницу:**
    *   Фронтенд-сервис (или Gateway) отдает статический HTML/JS виджет. В его основе лежит `business_id` из URL.
    *   Страница не требует аутентификации и является публичной.
3.  **Пользователь заполняет форму и отправляет данные:**
    *   Виджет отправляет AJAX POST-запрос на защищенный эндпоинт, например: `https://api.zero-dashboard.local/v1/frontend/review`.
    *   **Важно:** этот эндпоинт **не должен быть публичным**. Он должен быть защищен, чтобы только ваш фронтенд мог отправлять туда данные.

### Как защитить этот процесс? Основные угрозы и решения:

1.  **Угроза: Спам и поддельные запросы.** Злоумышленник может спамить ваш `/v1/frontend/review` эндпоинт, подделывая `business_id`.
    *   **Решение:** Используйте **JWT-токены с ограниченным сроком жизни**. При загрузке виджета, сервер генерирует подписанный JWT-токен, в котором зашит `business_id` и, например, `session_id`. Этот токен передается в JavaScript и отправляется обратно в заголовке `Authorization: Bearer` при каждом AJAX-запросе.
    *   **Реализация:**
        *   `session_id` (ULID) генерируется на бэкенде и передается в виджет.
        *   Сервер валидирует JWT на каждом запросе: проверяет подпись и срок действия.
        *   Это гарантирует, что запрос пришел с вашего виджета, а не от поддельного клиента.

2.  **Угроза: Подмена бизнес-идентификатора.** Пользователь может подменить `business_id` в запросе.
    *   **Решение:** При создании JWT-токена, `business_id` "запечатывается" в теле токена (claim). На бэкенде вы проверяете, что `business_id` из токена совпадает с тем, что передается в запросе. Это делает подмену невозможной, так как токен подписан секретным ключом.

3.  **Угроза: Подмена платформы.** Пользователь может отправлять отзывы, якобы из Telegram, через веб-виджет.
    *   **Решение:** Фронтенд-сервис сам определяет платформу (`platform`) на основе источника запроса (например, входящий URL или реферер). В вашем кейсе, веб-виджет всегда отправляет отзывы через `/v1/frontend/review`, и сервер явно задает `platform = "web"`. Это фиксировано в логике, и пользователь не может это изменить.

### Пример кода

**1. Новый сервис Frontend Service (Go):**

```go
// services/go/frontend/main.go (частичный пример)
package main

import (
    "context"
    "encoding/json"
    "fmt"
    "log"
    "net/http"
    "os"
    "time"
    "github.com/golang-jwt/jwt/v5"
    "github.com/nats-io/nats.go"
    "github.com/google/uuid" // Для генерации ULID можно использовать другую либу
)

type ReviewSubmission struct {
    BusinessID string `json:"business_id"`
    Platform   string `json:"platform"`
    Customer   Customer `json:"customer"`
    Message    Message `json:"message"`
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

// Генератор JWT при загрузке виджета
func handleWidget(w http.ResponseWriter, r *http.Request) {
    // 1. Извлекаем business_id из URL
    // businessID := extractBusinessIDFromPath(r.URL.Path) // например, /r/cafe_452

    // 2. Генерируем session_id (ULID)
    sessionID := generateULID()

    // 3. Создаем JWT
    token := jwt.NewWithClaims(jwt.SigningMethodHS256, jwt.MapClaims{
        "business_id": businessID,
        "session_id":  sessionID,
        "exp":         time.Now().Add(30 * time.Minute).Unix(), // Истекает через 30 мин
    })
    tokenString, err := token.SignedString([]byte(os.Getenv("JWT_SECRET")))
    if err != nil {
        http.Error(w, "Internal Server Error", http.StatusInternalServerError)
        return
    }

    // 4. Отдаем HTML-страницу с встроенным токеном
    // ... отдаем статику, подставляя {{.Token}} и {{.BusinessID}} в шаблон
}

// Эндпоинт для приема отзыва с виджета
func handleSubmitReview(w http.ResponseWriter, r *http.Request) {
    // 1. Извлекаем и валидируем JWT из заголовка Authorization
    tokenString := r.Header.Get("Authorization")
    token, err := jwt.Parse(tokenString, func(token *jwt.Token) (interface{}, error) {
        return []byte(os.Getenv("JWT_SECRET")), nil
    })

    if err != nil || !token.Valid {
        http.Error(w, "Unauthorized", http.StatusUnauthorized)
        return
    }

    claims, ok := token.Claims.(jwt.MapClaims)
    if !ok {
        http.Error(w, "Unauthorized", http.StatusUnauthorized)
        return
    }

    // 2. Извлекаем данные из тела запроса
    var req struct {
        Customer Customer `json:"customer"`
        Message  Message  `json:"message"`
    }
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        http.Error(w, "Invalid payload", http.StatusBadRequest)
        return
    }

    // 3. Обогащаем запрос данными из JWT
    businessID := claims["business_id"].(string)
    sessionID := claims["session_id"].(string)
    platform := "web" // Фиксированное значение

    // 4. Формируем сообщение для NATS
    // ... используем тот же InboundReviewMessage, что и в gateway
    inboundMessage := InboundReviewMessage{
        Metadata: Metadata{
            Traceparent: generateTraceparent(),
            Timestamp:   time.Now().Unix(),
        },
        Payload: ReviewPayload{
            SessionID:  sessionID,
            BusinessID: businessID,
            Customer:   req.Customer,
            Message:    req.Message,
        },
    }

    data, err := json.Marshal(inboundMessage)
    if err != nil {
        http.Error(w, "Internal Server Error", http.StatusInternalServerError)
        return
    }

    // 5. Публикуем в NATS
    subject := fmt.Sprintf("reviews.v1.inbound.%s.%s", platform, businessID)
    nc := getNatsConnection() // Получаем из глобального пула
    if err := nc.Publish(subject, data); err != nil {
        http.Error(w, "Internal Server Error", http.StatusInternalServerError)
        return
    }

    w.WriteHeader(http.StatusOK)
    json.NewEncoder(w).Encode(map[string]string{"status": "success"})
}
```

**2. Обновление Traefik/Reverse Proxy:**

Вам нужно добавить маршруты для нового сервиса.

```yaml
# deploy/nginx/nginx.conf или Traefik labels

# --- 4. Фронтенд виджета (отдача статики) ---
server {
    listen 80;
    server_name review.zero-dashboard.local;

    location /r/ {
        # Проксируем на бэкенд фронтенд-сервиса
        proxy_pass http://frontend-service:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# --- 5. API для AJAX-запросов ---
# Это часть вашего основного API-шлюза
server {
    listen 80;
    server_name api.zero-dashboard.local;

    location /v1/frontend/ {
        # Проксируем на тот же фронтенд-сервис
        proxy_pass http://frontend-service:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Другие эндпоинты...
}
```

### Интеграция с вашей архитектурой:

1.  **Защита публичных вебхуков:** Ваш `Integration Gateway` (публичные вебхуки) остается как есть. Он используется только для внешних систем (Telegram, SMS). Он **защищен секретным URL**, и NATS-заголовок `X-Business-ID` в нем **не нужен**, так как `business_id` зашит в URL и кладется в топик. (См. ваш `review_flow.md`).
2.  **Защита фронтенда:** Фронтенд-сервис использует `JWT` для защиты своего эндпоинта. `business_id` в NATS он кладет из валидированного JWT.
3.  **Общий конвейер:** И вебхуки, и фронтенд отправляют сообщения в один и тот же JetStream-стрим `inbound-reviews`. Ваш сервис `review-gating` (Rust) обрабатывает их одинаково.

### Резюме:

*   **Нужен новый сервис:** Отдельный `frontend-service` на Go.
*   **Защита:** Используйте **JWT с ограниченным сроком жизни** для AJAX-запросов.
*   **Схема работы:**
    1.  Загрузка виджета (`/r/{business-id}`): генерация JWT, встраивание в страницу.
    2.  Отправка отзыва (`POST /v1/frontend/review`): валидация JWT, извлечение `business_id`, отправка в NATS.
    3.  Никакого внешнего доступа: эндпоинт `/v1/frontend/review` не должен быть доступен извне без валидного JWT.
*   **Почему отдельный сервис:** Это позволяет:
    *   Независимо масштабировать фронтенд (больше памяти для статики) и API-шлюз (больше CPU для обработки вебхуков).
    *   Четко разделить ответственность: шлюз занимается интеграцией с внешним миром, фронтенд — обслуживанием пользователей.