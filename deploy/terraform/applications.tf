# ============================================================
# applications.tf — приложения, провайдеры, аутпост
# ============================================================

data "authentik_flow" "default_invalidation" {
  slug = "default-invalidation-flow"
}

data "authentik_flow" "default_authentication" {
  slug = "default-authentication-flow"
}

data "authentik_user" "akadmin" {
  username = "akadmin"
}

resource "authentik_provider_proxy" "gateway_provider" {
  name                = "gateway-proxy-provider"
  mode                = "proxy"
  external_host       = "http://api.${var.app_domain}"
  internal_host       = "http://gateway:8080"
  authorization_flow  = authentik_flow.magic_link_authz_flow.uuid
  authentication_flow = authentik_flow.magic_link_flow.uuid
  invalidation_flow   = data.authentik_flow.default_invalidation.id

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

resource "authentik_provider_proxy" "dashboard_provider" {
  name                = "dashboard-proxy-provider"
  mode                = "proxy"
  external_host       = "http://dash.${var.app_domain}"
  internal_host       = "http://dashboard-app:80"
  authorization_flow  = authentik_flow.magic_link_authz_flow.uuid
  authentication_flow = authentik_flow.magic_link_flow.uuid
  invalidation_flow   = data.authentik_flow.default_invalidation.id

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

# --- Policies ---
resource "authentik_policy_expression" "only_owners_allowed" {
  name       = "only-owners-allowed-policy"
  expression = <<-EOF
    # Если пользователь анонимный, не ломаем флоу, даем Authentik запустить аутентификацию
    if not request.user or request.user.is_anonymous:
        return True
        
    return request.user.groups.filter(name="role_owner").exists()
  EOF
}

resource "authentik_policy_binding" "bind_dashboard_policy" {
  target = authentik_application.dashboard_app.uuid
  policy = authentik_policy_expression.only_owners_allowed.id
  order  = 0
}

# Gateway only for role_owner and role_manager
resource "authentik_policy_expression" "owners_and_managers_allowed" {
  name       = "owners-and-managers-allowed-policy"
  expression = <<-EOF
    if not request.user or request.user.is_anonymous:
        return True
        
    return request.user.groups.filter(name__in=["role_owner", "role_manager"]).exists()
  EOF
}

resource "authentik_policy_binding" "bind_gateway_policy" {
  target = authentik_application.gateway_app.uuid
  policy = authentik_policy_expression.owners_and_managers_allowed.id
  order  = 0
}

# --- Outpost ---
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

# --- API Token for backend services (if needed) ---

resource "authentik_token" "backend_api_token" {
  identifier = "backend-api-token"
  intent     = "api"
  user       = data.authentik_user.akadmin.id
  expiring   = false
}
