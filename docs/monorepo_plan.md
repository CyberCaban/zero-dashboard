/reputation-platform (Корень монорепозитория)
│
├── .github/ # Автоматизация и CI/CD (GitHub Actions)
│ └── workflows/
│ ├── ci-go.yml # Тесты для Go (триггер по paths: 'services/go/**')
│ └── ci-rust.yml # Тесты для Rust (триггер по paths: 'services/rust/**')
│
├── api/ # ЕДИНЫЙ ИСТОЧНИК ПРАВДЫ ДЛЯ КОНТРАКТОВ
│ ├── protobuf/ # Если используете Protobuf (рекомендуется)
│ │ ├── common.proto
│ │ └── review.proto
│ └── json-schemas/ # Или строгие JSON-схемы для NATS-сообщений
│
├── deploy/ # ИНФРАСТРУКТУРА И РАЗВЕРТЫВАНИЕ
│ ├── docker-compose.infra.yml # NATS, Redis, Postgres, OTel, Jaeger
│ └── docker-compose.apps.yml # Наши сервисы (Gateway, Analyzer и т.д.)
│
├── gitops/ # КОНФИГУРАЦИЯ (Из прошлой задачи для ИИ)
│ ├── authentik/ # YAML Blueprints для Authentik
│ ├── traefik/ # Конфигурация динамических провайдеров / Middlewares
│ └── nats/ # Скрипты инициализации стримов JetStream
│
└── services/ # ИСХОДНЫЙ КОД СЕРВИСОВ
├── go/ # Зона Go (свой go.work или независимые модули)
│ ├── integration-gateway/ # Каждый сервис — изолированное Go-приложение
│ │ ├── cmd/
│ │ ├── go.mod
│ │ └── Dockerfile
│ └── review-gating/
│ ├── go.mod
│ └── Dockerfile
│
└── rust/ # Зона Rust (Cargo Workspace)
├── Cargo.toml # Корневой Cargo.toml для менеджмента воркспейса
├── sentiment-analyzer/ # Микросервис на Rust
│ ├── src/
│ ├── Cargo.toml
│ └── Dockerfile
└── platform-scraper/ # Скрейпер на Rust
├── src/
├── Cargo.toml
└── Dockerfile

1. **В зоне Rust (`/services/rust`)** используйте механизм **Cargo Workspaces**. В корневом `Cargo.toml` (который внутри папки `rust`, а не в самом корне репозитория) пропишите:

```toml
[workspace]
members = ["sentiment-analyzer", "platform-scraper"]

```

Это позволит Rust кэшировать общие зависимости (например, `tokio`, `serde`, `async-nats`) между вашими сервисами, ускоряя сборку в разы.

2. **В зоне Go (`/services/go`)** каждый сервис может жить со своим `go.mod`. Для локальной разработки в корне папки `go` можно временно инициализировать `go.work` (Go Workspaces), чтобы удобно переходить по коду между гейтвеем и логикой.

3. **В Dockerfile** сборку нужно запускать в контексте всего репозитория (чтобы сервисы имели доступ к папке `/api` для генерации кода из Protobuf), но кэшировать слои зависимостей раздельно.
