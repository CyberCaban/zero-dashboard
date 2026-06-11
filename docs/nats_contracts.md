# Спецификация контрактов данных и топиков шины NATS

**Платформа:** Zero-Dashboard Reputation Management System

**Формат данных:** JSON

**Стандарт трассировки:** W3C Trace Context (`traceparent`)

---

## 1. Общие принципы и архитектурные стандарты

- **Идентификация сессий:** Для сквозного отслеживания цепочки сообщений используется `session_id` в формате **ULID** (Universally Unique Lexicographically Sortable Identifier). Это гарантирует уникальность и сортировку по времени создания без накладных расходов.
- **Сквозной мониторинг:** Каждое сообщение внутри системы обязано содержать объект `metadata` с полем `traceparent`.
- **Формат заголовка traceparent:** `00-{trace_id}-{parent_span_id}-{flags}`.

### Сводный реестр топиков (Subjects)

| Название топика                              | Тип NATS     | Отправитель       | Получатель                 | Гарантия доставки            |
| -------------------------------------------- | ------------ | ----------------- | -------------------------- | ---------------------------- |
| `review.v1.inbound.<platform>.<business_id>` | JetStream    | Messenger Gateway | Review Gating              | At-Least-Once                |
| `sentiment.v1.analyze`                       | Core NATS    | Review Gating     | Sentiment Analyzer         | Request-Reply                |
| `review.v1.escalate.<business_id>`           | JetStream    | Review Gating     | Escalation Engine          | At-Least-Once                |
| `push.v1.dispatch.<channel>`                 | Core (Queue) | Any               | Push Service               | At-Most-Once (Load Balanced) |
| `merchant.v1.action.<business_id>`           | Core NATS    | Messenger Gateway | Review Gating / Escalation | Fire-and-Forget              |
| `scraper.v1.verify.<platform>`               | JetStream    | Review Gating     | Scraper Engine             | At-Least-Once (Delayed)      |
| `analytics.v1.session_closed`                | JetStream    | Review Gating     | Analytics/DB Worker        | At-Least-Once                |

---

## 2. Детальные схемы контрактов данных

### 2.1. Входящий отзыв от клиента

- **Топик:** `review.v1.inbound.<platform>.<business_id>`
- **Назначение:** Регистрация входящего текстового сообщения или оценки от пользователя из внешнего канала.

```json
{
  "metadata": {
    "traceparent": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
    "timestamp": 1781003421
  },
  "payload": {
    "session_id": "01HZF7B6Z8E5V9XW42A7Q9P123",
    "business_id": "cafe_452",
    "customer": {
      "platform_user_id": "tg_user_998231",
      "phone": "+79991234567",
      "name": "Иван Иванов"
    },
    "message": {
      "message_id": "msg_88231",
      "text": "Принесли холодный суп, официант хамил и долго не нес счет!",
      "raw_rating": 2
    }
  }
}
```

---

### 2.2. Анализ тональности и тегирование (Request-Reply)

- **Топик:** `sentiment.v1.analyze`
- **Назначение:** Высоконагруженный синхронный запрос из Go-сервиса в Rust-парсер для извлечения сущностей и оценки тональности.

#### Схема запроса (Go -> Rust):

```json
{
  "metadata": {
    "traceparent": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
  },
  "text": "Принесли холодный суп, официант хамил и долго не нес счет!"
}
```

#### Схема ответа (Rust -> Go):

```json
{
  "sentiment": "NEGATIVE",
  "confidence": 0.98,
  "topics": ["kitchen", "service"],
  "is_actionable": true
}
```

---

### 2.3. Боевая тревога и эскалация негатива

- **Топик:** `review.v1.escalate.<business_id>`
- **Назначение:** Запуск таймера SLA и фиксация инцидента при обнаружении критического отзыва.

```json
{
  "metadata": {
    "traceparent": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
    "timestamp": 1781003422
  },
  "escalation_id": "01HZF7C2Y9M3N8V654B1A0X789",
  "business_id": "cafe_452",
  "session_id": "01HZF7B6Z8E5V9XW42A7Q9P123",
  "sentiment_data": {
    "rating": 2,
    "text": "Принесли холодный суп, официант хамил...",
    "detected_issues": ["kitchen", "service"]
  },
  "sla_seconds": 600
}
```

---

### 2.4. Диспетчер исходящих пушей и сообщений

- **Топик:** `push.v1.dispatch.<channel>`
- **Назначение:** Команда для Push Notification Service на отправку UI-элементов или текста конечному адресату.

```json
{
  "metadata": {
    "traceparent": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
  },
  "recipient": {
    "target_id": "tg_chat_5541223",
    "phone": "+79991234567"
  },
  "content": {
    "text": "🚨 НЕГАТИВ! Стол №3. Холодный суп. Официант хамил.",
    "inline_keyboard": [
      [
        {
          "text": "📞 Позвонить",
          "callback_data": "action_call_01HZF7B6Z8E5V9XW42A7Q9P123"
        },
        {
          "text": "💬 Ответить",
          "callback_data": "action_reply_01HZF7B6Z8E5V9XW42A7Q9P123"
        }
      ],
      [
        {
          "text": "🎁 Дать скидку 20%",
          "callback_data": "action_discount_20_01HZF7B6Z8E5V9XW42A7Q9P123"
        }
      ]
    ]
  }
}
```

---

### 2.5. Действие или ответ мерчанта

- **Топик:** `merchant.v1.action.<business_id>`
- **Назначение:** Передача ответа владельца бизнеса обратно в логику сценария (FSM) для маршрутизации клиенту.

```json
{
  "metadata": {
    "traceparent": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
    "timestamp": 1781003600
  },
  "business_id": "cafe_452",
  "session_id": "01HZF7B6Z8E5V9XW42A7Q9P123",
  "manager_id": "tg_user_77123",
  "action_type": "TEXT_REPLY",
  "text_payload": "Иван, простите ради бога! Я управляющий. Суп убрали из счета...",
  "bonus_granted": null
}
```

---

### 2.6. Валидация публикации внешнего отзыва

- **Топик:** `scraper.v1.verify.<platform>`
- **Назначение:** Передача таски в Rust Scraper Engine для пост-проверки факта размещения отзыва на картах.

```json
{
  "metadata": {
    "traceparent": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
  },
  "task_id": "task_scrap_9912",
  "session_id": "01HZF7B6Z8E5V9XW42A7Q9P123",
  "target_platform_id": "gmb_place_id_uue89231",
  "customer_name_variant": "Иван Иванов",
  "expected_text_keywords": ["суп", "официант"]
}
```

---

### 2.7. Закрытие сессии (Аналитический лог)

- **Топик:** `analytics.v1.session_closed`
- **Назначение:** Финализация сессии диалога. Вычитывается воркером долгосрочного хранения данных (PostgreSQL/ClickHouse).

```json
{
  "session_id": "01HZF7B6Z8E5V9XW42A7Q9P123",
  "business_id": "cafe_452",
  "created_at": 1781003421,
  "closed_at": 1781003800,
  "initial_rating": 2,
  "sentiment_tags": ["kitchen", "service"],
  "gating_status": "INTERCEPTED_AND_RESOLVED",
  "merchant_response_time_seconds": 179,
  "total_messages_exchanged": 4
}
```

---

> **Важное примечание для разработчиков:** Изменение структуры любого из вышеперечисленных JSON-файлов требует обязательного мажорного инкремента версии в названии топика (например, с `v1` на `v2`), чтобы не нарушить обратную совместимость между Go и Rust сервисами в проде.
