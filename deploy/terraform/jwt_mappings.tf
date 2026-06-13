# ============================================================
# jwt_mappings.tf — проброс B2B-контекста в заголовки
#
# Authentik proxy forward-auth передаёт атрибуты пользователя
# бэкенду через X-authentik-* заголовки.
# Scope-маппинг определяет какие поля туда попадают.
# ============================================================

resource "authentik_property_mapping_provider_scope" "b2b_context" {
  name       = "b2b-tenant-context"
  scope_name = "reputation_b2b"

  expression = <<-EOF
    # ak_groups — правильный менеджер групп в Authentik
    # user.groups — Django ORM relation, работает только в некоторых контекстах
    return {
        "tenant_id":         user.attributes.get("tenant_id", "unknown_tenant"),
        "allowed_locations": user.attributes.get("allowed_locations", []),
        "roles":             [g.name for g in request.user.groups.all()],
    }
  EOF
}
