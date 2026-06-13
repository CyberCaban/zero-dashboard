import sys
import requests
import uuid
from datetime import datetime, timedelta, timezone

# --- КОНФИГУРАЦИЯ ---
AUTHENTIK_URL = "http://authentik.local"
API_TOKEN = "Hj0ZJtdhrsFfauowfccnNqQ5qjvE5qdWkUFo6dzeSCcppADl2xgF5jgkdKLw" 
APP_DOMAIN = "zero-dashboard.local"
FLOW_SLUG = "custom-magic-link" 

def create_magic_link(email, tenant_id, location):
    # Изменили эндпоинт на создание обычных токенов сессий/API
    url = f"{AUTHENTIK_URL}/api/v3/core/tokens/"
    
    headers = {
        "Authorization": f"Bearer {API_TOKEN}",
        "Content-Type": "application/json"
    }
    
    expires_at = (datetime.now(timezone.utc) + timedelta(days=7)).strftime("%Y-%m-%dT%H:%M:%SZ")
    
    # Генерируем уникальный secure-идентификатор для URL
    unique_token_id = f"magic-{uuid.uuid4()}"
    
    payload = {
        "identifier": unique_token_id,
        "intent": "api", # Тип токена - API/внешний
        "user": 1,       # От чьего имени создается (аккаунт администратора)
        "description": f"Magic link token for {email}",
        "expiring": True,
        "expires": expires_at,
        "attributes": {
            "email": email,
            "tenant_id": tenant_id,
            "allowed_locations": [location]
        }
    }
    
    response = requests.post(url, json=payload, headers=headers)
    
    if response.status_code == 201:
        # Токен успешно создан в БД Authentik
        magic_link = f"http://dash.{APP_DOMAIN}/if/flow/{FLOW_SLUG}/?itoken={unique_token_id}&next=http://dash.{APP_DOMAIN}/"
        return magic_link
    else:
        print(f"Ошибка Authentik API: {response.status_code}")
        print(response.text)
        return None

if __name__ == "__main__":
    if len(sys.argv) < 4:
        print("Использование: python generate_magic_link.py <email> <tenant_id> <location>")
        sys.exit(1)
        
    user_email = sys.argv[1]
    user_tenant = sys.argv[2]
    user_location = sys.argv[3]
    
    print(f"Генерируем ссылку для {user_email} (Tenant: {user_tenant})...")
    link = create_magic_link(user_email, user_tenant, user_location)
    
    if link:
        print("\n ССЫЛКА СГЕНЕРИРОВАНА УСПЕШНО:")
        print(link)