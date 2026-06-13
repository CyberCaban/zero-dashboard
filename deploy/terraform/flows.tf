# 1. Сам Флоу для беспарольного входа
resource "authentik_flow" "magic_link_flow" {
  name               = "Zero-Dashboard Magic Link Flow"
  slug               = "custom-magic-link"
  title              = "Вход по одноразовой ссылке"
  designation        = "authentication" 
  compatibility_mode = true
  authentication     = "none"
  layout             = "sidebar_left"
}

# 2. Стадия автоматического логина
resource "authentik_stage_user_login" "auto_login" {
  name = "magic-auto-login-stage"
}

# 3. Привязываем логин к флоу
resource "authentik_flow_stage_binding" "bind_login_stage" {
  target = authentik_flow.magic_link_flow.uuid
  stage  = authentik_stage_user_login.auto_login.id
  order  = 10
}

# 4. Мощная и отказоустойчивая политика авторизации по Токену
resource "authentik_policy_expression" "bridge_invite_to_login" {
  name       = "bridge-invite-to-login-policy"
  expression = <<EOF
ak_logger.info("============== PURE TOKEN MAGIC LINK START ==============")

http_req = context.get("http_request")
if http_req:
    query_string = http_req.META.get('QUERY_STRING', '')
    itoken = http_req.GET.get("itoken")
    
    if not itoken and "itoken=" in query_string:
        import urllib.parse
        params = urllib.parse.parse_qs(query_string)
        itoken = params.get("itoken", [None])[0]

    if itoken:
        ak_logger.info(f"Magic Link Policy processing token: {itoken}")
        from authentik.core.models import Token
        from datetime import datetime, timezone
        
        # Ищем токен в базе данных Authentik
        db_token = Token.objects.filter(identifier=itoken).first()
        
        if db_token:
            # Проверяем, не истек ли токен
            if db_token.expires and db_token.expires < datetime.now(timezone.utc):
                ak_logger.warn(f"Token {itoken} has expired!")
                return False
                
            # Достаем email из атрибутов токена
            invite_email = db_token.attributes.get("email")
            ak_logger.info(f"Token resolved to email: {invite_email}")
            
            if invite_email:
                from authentik.core.models import User
                user = User.objects.filter(email=invite_email).first()
                if user:
                    ak_logger.info(f"Injected user {user.username} successfully into flow context.")
                    
                    # Передаем юзера в стадию логина
                    context["pending_user"] = user
                    
                    # Опционально: если ссылка должна быть СТРОГО одноразовой, 
                    # удаляем токен сразу после успешного логина:
                    # db_token.delete()
                    
                    return True

ak_logger.error("Magic Link Policy failed to authenticate user")
return False
EOF
}

# 5. Привязываем политику к стадии логина
resource "authentik_policy_binding" "bind_bridge_policy" {
  target = authentik_flow_stage_binding.bind_login_stage.id
  policy = authentik_policy_expression.bridge_invite_to_login.id
  order  = 0
}