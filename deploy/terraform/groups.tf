resource "authentik_group" "role_owner" {
  name         = "role_owner"
  is_superuser = false
  attributes = jsonencode({
    description = "Business owner. Full access to dashboard"
  })
}

resource "authentik_group" "role_manager" {
  name = "role_manager"
  is_superuser = false
  attributes = jsonencode({
    description = "Shift manager. Can manage client chats"
  })
}