Отзыв

1. Вебхук от Telegram (Telegram → Наша система)
   Когда клиент пишет в бота кафе, Telegram шлёт POST-запрос на наш сервер:

```http
POST {{host}}/v1/webhooks/telegram/SECRET_TOKEN_CAFE_452
Content-Type: application/json

{
  "payload": {
    "customer": {
      "platform_user_id": "tg_user_998231",
      "phone": "+79991234567",
      "name": "Иван Иванов"
    },
    "message": {
      "message_id": "msg_88231",
      "text": "Принесли холодный суп, официант хамил!",
      "raw_rating": 2
    }
  }
}
```

business_id зашит в самом URL в виде секретного токена.
Почему так? Потому что у каждого кафе — свой отдельный Telegram-бот (или свой токен бота). Мы при регистрации кафе в базе сохраняем:

```
businesses:
  cafe_452 → telegram_bot_token = "7f3a...xyz"
  salon_777 → telegram_bot_token = "9b1c...abc"
```

Когда Telegram дёргает webhook, Go Gateway:
Извлекает токен из URL: 7f3a...xyz
Ищет в Postgres: «Какому business_id принадлежит этот токен?» → cafe_452
Кладёт business_id в NATS-заголовок: msg.Header.Set("X-Business-ID", "cafe_452")

Никакой авторизации через Authentik тут нет! Это публичный webhook, защищённый секретным URL

2. Клиент оставляет отзыв (Клиент → Наша система)
   Клиент не ходит на /review/telegram. Он взаимодействует иначе:

Вариант А: QR-код на столе
На столе в кафе лежит QR-код:

```curl
https://review.нашсервис.com/r/cafe_452?session=01HZF7B6Z8...
```

business_id (cafe_452) зашит прямо в URL. Клиент сканирует → попадает на веб-виджет → ставит оценку → виджет шлёт POST на наш API с этим ID.

Вариант Б: Через Telegram-бота
Клиент пишет боту /start → бот присылает кнопку «Оценить визит» → клиент жмёт → бот получает callback_query через тот же webhook из Потока 1. В теле callback уже есть chat_id клиента и message_id — по ним в Redis достаётся активная сессия с business_id.
