resource "authentik_property_mapping_provider_scope" "b2b_context" {
  name = "b2b-tenant-context"
    scope_name = "reputation_b2b"

    expression = <<EOF
        return {
            "tenant_id": user.attributes.get("tenant_id", "unknown_tenant"),
            "allowed_locations": user.attributes.get("allowed_locations", []),
            "roles": [group.name for group in user.ak_groups.all()]
        }
    EOF
}