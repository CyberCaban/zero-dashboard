# ============================================================
# flow_authz.tf — проверка прав доступа (Authorization Flow)
# ============================================================

resource "authentik_flow" "magic_link_authz_flow" {
  name               = "Zero-Dashboard Authorization Flow"
  slug               = "custom-dashboard-authorization"
  title              = "Проверка прав доступа"
  designation        = "authorization"
  compatibility_mode = true
}

resource "authentik_policy_expression" "user_authz_check" {
  name       = "dashboard-authorization-policy"
  
  expression = <<-EOF
    user = request.user
    
    if not user or not user.is_active:
        ak_logger.warning(f"Authz denied: user is inactive or anonymous")
        return False
        
    # 2. Проверяем роль/группу (поскольку в твоем описании упоминалась "нужная роль")
    # Предположим, у тебя есть группа "role_owner". Если юзера там нет — доступ закрыт.
    # Чтобы активировать проверку, просто раскомментируй строки ниже:
    #
    if not user.groups.filter(name="role_owner").exists():
        ak_logger.warning(f"Authz denied: user {user.email} does not have the required role")
        return False
        
    ak_logger.info(f"Authz allowed: user {user.email} passed all checks")
    return True
  EOF
}

resource "authentik_policy_binding" "bind_authz_check_to_flow" {
  target = authentik_flow.magic_link_authz_flow.uuid
  policy = authentik_policy_expression.user_authz_check.id
  order  = 0
}