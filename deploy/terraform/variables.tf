variable "authentik_url" {
  type = string
  default = "http://localhost:80"
  description = "Deployed Authentik URL"
}

variable "authentik_bootstrap_token" {
  type = string
  sensitive = true
  description = "Sudo token for Authentik, used for initial setup and configuration"
}

variable "app_domain" {
  type = string
  default = "zero-dashboard.local"
  description = "Platform domain"
}