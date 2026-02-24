resource "cloudflare_pages_project" "website" {
  account_id        = var.cloudflare_account_id
  name              = var.pages_project_name
  production_branch = var.production_branch
}

# Custom domain — auto-creates CNAME for zones already on Cloudflare
resource "cloudflare_pages_domain" "website" {
  account_id   = var.cloudflare_account_id
  project_name = cloudflare_pages_project.website.name
  name         = var.custom_domain
}
