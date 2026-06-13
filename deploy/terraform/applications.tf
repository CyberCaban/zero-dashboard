data "authentik_flow" "default_invalidation" {
  slug = "default-invalidation-flow"
}

data "authentik_flow" "default_authentication" {
  slug = "default-authentication-flow"
}

# --- Операционный Слои (Gateway) ---
resource "authentik_provider_proxy" "gateway_provider" {
  name               = "gateway-proxy-provider"
  mode               = "forward_single"
  external_host      = "https://api.${var.app_domain}"
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

# --- Стратегический Слой (Dashboard) ---
resource "authentik_provider_proxy" "dashboard_provider" {
  name               = "dashboard-proxy-provider"
  mode               = "forward_single"
  external_host      = "http://dash.${var.app_domain}"
  internal_host      = "http://dashboard-app:80"
  invalidation_flow  = data.authentik_flow.default_invalidation.id
  authorization_flow = data.authentik_flow.default_authentication.id

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

# --- Политики ---
resource "authentik_policy_expression" "only_owners_allowed" {
  name       = "only-owners-allowed-policy"
  expression = "return 'role_owner' in [g.name for g in request.user.ak_groups.all()]"
}

resource "authentik_policy_binding" "bind_dashboard_policy" {
  target = authentik_application.dashboard_app.uuid
  policy = authentik_policy_expression.only_owners_allowed.id
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
    authentik_provider_proxy.dashboard_provider.id
  ]
}

resource "authentik_token" "backend_api_token" {
  identifier = "backend-magic-link-generator"
  intent     = "api"
  user       = 1
}