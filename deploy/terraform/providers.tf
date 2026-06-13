terraform {
    required_providers {
        authentik = {
            source = "goauthentik/authentik"
            version = "= 2026.5.0"
        }
    }
}

provider "authentik" {
    url = var.authentik_url
    token = var.authentik_bootstrap_token
}