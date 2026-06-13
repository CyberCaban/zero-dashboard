# ============================================================
# flows.tf — беспарольный вход через Email magic link
# ============================================================

# --- 1. Основной Flow (запрос ссылки) ---
resource "authentik_flow" "magic_link_flow" {
  name               = "Zero-Dashboard Magic Link Flow"
  slug               = "custom-magic-link"
  title              = "Вход по одноразовой ссылке"
  designation        = "authentication"
  compatibility_mode = true
  authentication     = "none"
  layout             = "sidebar_left"
}

# --- 2. Activation Flow (переход по ссылке из письма) ---
resource "authentik_flow" "magic_link_activation_flow" {
  name               = "Magic Link Activation Flow"
  slug               = "custom-magic-link-activation"
  title              = "Активация по ссылке"
  designation        = "authentication"
  authentication     = "none"
  compatibility_mode = false
}

# --- 3. Identification Stage для Activation Flow ---
resource "authentik_stage_identification" "activation_id_stage" {
  name            = "magic-activation-identification-stage"
  user_fields     = ["email"]
  enrollment_flow = null
  recovery_flow   = null
}

# --- 4. User Login Stage для Activation Flow ---
resource "authentik_stage_user_login" "activation_login" {
  name                     = "magic-activation-login-stage"
  terminate_other_sessions = true
}

# --- 5. Redirect Stage для Activation Flow ---
resource "authentik_stage_redirect" "activation_redirect" {
  name          = "magic-activation-redirect-stage"
  keep_context  = false
  mode          = "static"
  target_static = "http://dash.${var.app_domain}"
}

# --- 6. Привязка стадий к Activation Flow ---
resource "authentik_flow_stage_binding" "bind_activation_id" {
  target = authentik_flow.magic_link_activation_flow.uuid
  stage  = authentik_stage_identification.activation_id_stage.id
  order  = 0
}

resource "authentik_flow_stage_binding" "bind_activation_login" {
  target = authentik_flow.magic_link_activation_flow.uuid
  stage  = authentik_stage_user_login.activation_login.id
  order  = 10
}

resource "authentik_flow_stage_binding" "bind_activation_redirect" {
  target = authentik_flow.magic_link_activation_flow.uuid
  stage  = authentik_stage_redirect.activation_redirect.id
  order  = 20
}

# --- 7. Остальные стадии для основного Flow ---

# Identification Stage для основного flow
resource "authentik_stage_identification" "email_id_stage" {
  name            = "magic-link-identification-stage"
  user_fields     = ["email"]
  enrollment_flow = null
  recovery_flow   = null
}

# Email Stage для отправки письма
resource "authentik_stage_email" "magic_link_email_stage" {
  name                = "magic-link-email-stage"
  use_global_settings = true
  template            = "email/password_reset.html"
  token_expiry        = "minutes=30"
}

# Prompt Stage "Проверьте почту"
resource "authentik_stage_prompt_field" "check_email_prompt" {
  name        = "magic-link-check-email-prompt"
  field_key   = "check_email_message"
  label       = "Проверьте почту"
  placeholder = "Мы отправили ссылку для входа на ваш email. Перейдите по ссылке из письма."
  type        = "static"
  required    = false
}

resource "authentik_stage_prompt" "check_email_stage" {
  name   = "magic-link-check-email-stage"
  fields = [authentik_stage_prompt_field.check_email_prompt.id]
}

# --- 8. Привязка стадий к основному Flow ---
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

resource "authentik_flow_stage_binding" "bind_check_email_stage" {
  target = authentik_flow.magic_link_flow.uuid
  stage  = authentik_stage_prompt.check_email_stage.id
  order  = 20
}