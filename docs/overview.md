                                  +---------------------------------------+
                                  |            REVERSE PROXY              |
                                  |         (Traefik / Nginx)             |
                                  +---+-------------------------------+---+
                                      |                               |
                                      | (SMS Webhooks)                | (OIDC / OAuth2)
                                      v                               v
                         +--------------------------+    +--------------------------+
                         |                          |    |      Identity & AAA      |
                         |   Integration Gateway    |    |   (Authentik / Zitadel)  |
                         |          (Go)            |    +--------------------------+
                         +------------+-------------+
                                      |
                                      | (NATS Request-Reply / Kafka)
                                      v
                         +--------------------------+
                         |      Message Broker      |
                         |    (NATS / Kafka DB)     |
                         +---+----+----+----+----+--+
                             |    |    |    |    |
           +-----------------+    |    |    |    +-----------------+
           | (Queue Group)        |    |    | (Queue Group)        | (Pub/Sub)
           v                      |    v    v                      v
+--------------------+            |  +--------------------+  +--------------------+
|   Review Gating    |            |  | Alerting & Escal.  |  |   Multi-Platform   |
|      Service       |            |  |     Engine         |  |   Scraper Engine   |
|       (Go)         |            |  |      (Go)          |  |       (Rust)       |
+---------+----------+            |  +--------------------+  +--------------------+
          |                       |
          | (Request-Reply)       | (Pub/Sub / Push Event)
          v                       v
+--------------------+  +--------------------+
| Sentiment & Topic  |  | Push Notification  |
|     Analyzer       |  |     Service        |
|      (Rust)        |  |       (Go)         |
+--------------------+  +---------+----------+
                                  |
                                  | (SMS / Telegram APIs)
                                  v
                        +--------------------+
                        |  Mobile / Telegram |
                        |     Networks       |
                        +--------------------+