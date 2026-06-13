# ============================================================
# applications.tf — приложения, провайдеры, аутпост
# ============================================================

data "authentik_flow" "default_invalidation" {
  slug = "default-invalidation-flow"
}

data "authentik_flow" "default_authentication" {
  slug = "default-authentication-flow"
}

# Пользователь akadmin (ID=1 — ненадёжно; ищем по username)
data "authentik_user" "akadmin" {
  username = "akadmin"
}

# --- Операционный слой (Gateway) ---

resource "authentik_provider_proxy" "gateway_provider" {
  name               = "gateway-proxy-provider"
  mode               = "forward_single"
  external_host      = "http://api.${var.app_domain}"
  internal_host      = "http://gateway:8080"
  authorization_flow = authentik_flow.magic_link_flow.uuid
  invalidation_flow  = data.authentik_flow.default_invalidation.id

  property_mappings = [
    authentik_property_mapping_provider_scope.b2b_context.id
  ]
}

resource "authentik_application" "gateway_app" {
  name               = "Zero-Dashboard Operational Gateway"
  slug               = "ops-gateway"
  protocol_provider  = authentik_provider_proxy.gateway_provider.id
  policy_engine_mode = "any"
  meta_launch_url    = "http://api.${var.app_domain}"
}

# --- Стратегический слой (Dashboard) ---

resource "authentik_provider_proxy" "dashboard_provider" {
  name               = "dashboard-proxy-provider"
  mode               = "forward_single"
  external_host      = "http://dash.${var.app_domain}"
  internal_host      = "http://dashboard-app:80"
  authorization_flow = authentik_flow.magic_link_flow.uuid
  invalidation_flow  = data.authentik_flow.default_invalidation.id

  property_mappings = [
    authentik_property_mapping_provider_scope.b2b_context.id
  ]
}

resource "authentik_application" "dashboard_app" {
  name              = "Zero-Dashboard Strategic Analytics"
  slug              = "strategic-dashboard"
  protocol_provider = authentik_provider_proxy.dashboard_provider.id
  meta_launch_url   = "http://dash.${var.app_domain}"
}

# --- Политики доступа ---

# Дашборд доступен только role_owner
resource "authentik_policy_expression" "only_owners_allowed" {
  name       = "only-owners-allowed-policy"
  expression = "return 'role_owner' in [g.name for g in request.user.ak_groups.all()]"
}

resource "authentik_policy_binding" "bind_dashboard_policy" {
  target = authentik_application.dashboard_app.uuid
  policy = authentik_policy_expression.only_owners_allowed.id
  order  = 0
}

# Gateway доступен role_owner и role_manager
resource "authentik_policy_expression" "owners_and_managers_allowed" {
  name       = "owners-and-managers-allowed-policy"
  expression = <<-EOF
    allowed = {"role_owner", "role_manager"}
    user_roles = {g.name for g in request.user.ak_groups.all()}
    return bool(allowed & user_roles)
  EOF
}

resource "authentik_policy_binding" "bind_gateway_policy" {
  target = authentik_application.gateway_app.uuid
  policy = authentik_policy_expression.owners_and_managers_allowed.id
  order  = 0
}

# --- Аутпост ---

resource "authentik_outpost" "zero_dashboard_outpost" {
  name = "zero-dashboard-proxy-outpost"
  type = "proxy"

  config = jsonencode({
    authentik_host          = "http://server:9000"
    authentik_host_insecure = true
    log_level               = "info"
  })

  protocol_providers = [
    authentik_provider_proxy.gateway_provider.id,
    authentik_provider_proxy.dashboard_provider.id,
  ]
}

# --- API Token для бэкенда (через data source, без hardcode ID) ---

resource "authentik_token" "backend_api_token" {
  identifier = "backend-api-token"
  intent     = "api"
  user       = data.authentik_user.akadmin.id
  expiring   = false
}
