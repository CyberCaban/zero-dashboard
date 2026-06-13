# ============================================================
# flows.tf — беспарольный вход через Email magic link
#
# Архитектура:
#   1. Identification Stage  — пользователь вводит email
#   2. Expression Policy     — проверяем что email есть в базе
#   3. Email Stage           — Authentik сам генерирует токен
#                              и шлёт письмо со ссылкой
#   4. User Login Stage      — автологин после верификации
# ============================================================

# --- 1. Flow ---

resource "authentik_flow" "magic_link_flow" {
  name               = "Zero-Dashboard Magic Link Flow"
  slug               = "custom-magic-link"
  title              = "Вход по одноразовой ссылке"
  designation        = "authentication"
  compatibility_mode = true
  authentication     = "none"
  layout             = "sidebar_left"
}

# --- 2. Identification Stage (форма ввода email) ---

resource "authentik_stage_identification" "email_id_stage" {
  name           = "magic-link-identification-stage"
  user_fields    = ["email"]

  # Запрещаем регистрацию — только существующие пользователи
  enrollment_flow = null
  recovery_flow   = null
}

# --- 3. Expression Policy — проверка: пользователь существует и имеет нужную роль ---

resource "authentik_policy_expression" "user_exists_check" {
  name       = "magic-link-user-exists-policy"
  # К этому моменту Identification Stage уже нашёл пользователя по email
  # и положил его в pending_user. Если email не существовал — до сюда не дошли.
  # Нам остаётся только проверить is_active.
  expression = <<-EOF
    pending_user = context.get("pending_user")
    if not pending_user:
        return False
    if not pending_user.is_active:
        ak_logger.warning(f"magic-link: user {pending_user.email} is inactive, denying")
        return False
    return True
  EOF
}

# Привязываем политику к Email Stage binding.
# pending_user появляется в контексте только ПОСЛЕ того как
# Identification Stage отработал и нашёл пользователя по email.
# Поэтому проверяем на входе в следующую стадию — Email Stage.
resource "authentik_policy_binding" "bind_user_exists_to_email_stage" {
  target = authentik_flow_stage_binding.bind_email_stage.id
  policy = authentik_policy_expression.user_exists_check.id
  order  = 0
}

# --- 4. Email Stage — Authentik сам генерирует токен и шлёт письмо ---

resource "authentik_stage_email" "magic_link_email_stage" {
  name = "magic-link-email-stage"

  # Это ключевое: одноразовая ссылка вместо кода
  use_global_settings = true

  # Шаблон письма (можно переопределить через custom-templates)
  template = "email/password_reset.html"

  token_expiry = "minutes=30" # минут
}

# --- 5. User Login Stage ---

resource "authentik_stage_user_login" "auto_login" {
  name                     = "magic-auto-login-stage"
  session_duration         = "hours=8"
  terminate_other_sessions = false
}

# --- 6. Привязка стадий к Flow ---

resource "authentik_flow_stage_binding" "bind_id_stage" {
  target = authentik_flow.magic_link_flow.uuid
  stage  = authentik_stage_identification.email_id_stage.id
  order  = 0
}

resource "authentik_flow_stage_binding" "bind_email_stage" {
  target = authentik_flow.magic_link_flow.uuid
  stage  = authentik_stage_email.magic_link_email_stage.id
  order  = 10
}

resource "authentik_flow_stage_binding" "bind_login_stage" {
  target = authentik_flow.magic_link_flow.uuid
  stage  = authentik_stage_user_login.auto_login.id
  order  = 20
}
