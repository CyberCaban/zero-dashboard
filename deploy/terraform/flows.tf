data "authentik_flow" "default_login" {
  slug = "default-authentication-flow"
}

resource "authentik_flow" "magic_link_flow" {
  name               = "Zero-Dashboard Magic Link Flow"
  slug               = "custom-magic-link"
  title              = "Вход по одноразовой ссылке"
  designation        = "authentication"
  compatibility_mode = true
}

resource "authentik_stage_invitation" "magic_token_verification" {
  name          = "magic-token-verification-stage"
  continue_flow_without_invitation = false
}

resource "authentik_flow_stage_binding" "bind_token_stage" {
  target = authentik_flow.magic_link_flow.uuid
  stage  = authentik_stage_invitation.magic_token_verification.id
  order  = 10
}

resource "authentik_stage_user_login" "auto_login" {
  name = "magic-auto-login-stage"
}

resource "authentik_flow_stage_binding" "bind_login_stage" {
  target = authentik_flow.magic_link_flow.uuid
  stage  = authentik_stage_user_login.auto_login.id
  order  = 20
}