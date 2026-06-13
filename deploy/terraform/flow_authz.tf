# ============================================================
# flows.tf — проверка прав доступа (Authorization Flow)
#
# Архитектура:
#   1. Authorization Flow — запускается автоматически при переходе в приложение
#   2. Expression Policy   — проверяет статус и права уже вошедшего пользователя
#   3. Policy Binding      — привязывает логику проверки напрямую к потоку
# ============================================================

# --- 1. Flow ---

resource "authentik_flow" "magic_link_authz_flow" {
  name               = "Zero-Dashboard Authorization Flow"
  slug               = "custom-dashboard-authorization"
  title              = "Проверка прав доступа"
  designation        = "authorization" # <- Главное отличие: теперь это поток авторизации
  compatibility_mode = true
}

# --- 2. Expression Policy — проверка прав текущего пользователя ---

resource "authentik_policy_expression" "user_authz_check" {
  name       = "dashboard-authorization-policy"
  
  # В контексте авторизации пользователь уже вошел, поэтому мы работаем 
  # с request.user (а не с pending_user, как было на этапе входа).
  expression = <<-EOF
    user = request.user
    
    # 1. Проверяем, что пользователь вообще существует и активен
    if not user or not user.is_active:
        ak_logger.warning(f"Authz denied: user is inactive or anonymous")
        return False
        
    # 2. Проверяем роль/группу (поскольку в твоем описании упоминалась "нужная роль")
    # Предположим, у тебя есть группа "dashboard-users". Если юзера там нет — доступ закрыт.
    # Чтобы активировать проверку, просто раскомментируй строки ниже:
    #
    # if not user.groups.filter(name="dashboard-users").exists():
    #     ak_logger.warning(f"Authz denied: user {user.email} does not have the required role")
    #     return False
        
    return True
  EOF
}

# --- 3. Привязка политики напрямую к Flow ---
# В потоках авторизации нет визуальных стадий (stages), поэтому 
# политика вешается прямо на сам Flow.

resource "authentik_policy_binding" "bind_authz_check_to_flow" {
  target = authentik_flow.magic_link_authz_flow.uuid
  policy = authentik_policy_expression.user_authz_check.id
  order  = 0
}