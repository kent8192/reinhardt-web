resource "cloudflare_pages_project" "website" {
  account_id        = var.cloudflare_account_id
  name              = var.pages_project_name
  production_branch = var.production_branch
}

locals {
  pages_subdomain = "${cloudflare_pages_project.website.name}.pages.dev"
  rc_branch_slug  = replace(lower(var.rc_branch_name), "/[^0-9a-z]/", "-")
}

# Custom domain — registers domain with the Pages project
resource "cloudflare_pages_domain" "website" {
  account_id   = var.cloudflare_account_id
  project_name = cloudflare_pages_project.website.name
  name         = var.custom_domain
}

# Look up the existing Cloudflare zone for the custom domain
data "cloudflare_zone" "website" {
  filter = {
    name = var.custom_domain
  }
}

# Apex domain CNAME record pointing to Cloudflare Pages
resource "cloudflare_dns_record" "website_apex" {
  zone_id = data.cloudflare_zone.website.zone_id
  name    = "@"
  type    = "CNAME"
  content = local.pages_subdomain
  ttl     = 1
  proxied = true

  depends_on = [cloudflare_pages_domain.website]
}

# Custom domain for www subdomain — registers www with the Pages project
resource "cloudflare_pages_domain" "pages_website_www" {
  account_id   = var.cloudflare_account_id
  project_name = cloudflare_pages_project.website.name
  name         = "www.${var.custom_domain}"
}

# www subdomain CNAME record pointing to Cloudflare Pages
resource "cloudflare_dns_record" "dns_website_www" {
  zone_id = data.cloudflare_zone.website.zone_id
  name    = "www"
  type    = "CNAME"
  content = local.pages_subdomain
  ttl     = 1
  proxied = true

  depends_on = [cloudflare_pages_domain.pages_website_www]
}

resource "cloudflare_pages_domain" "pages_website_notes" {
  account_id   = var.cloudflare_account_id
  project_name = cloudflare_pages_project.website.name
  name         = "notes.${var.custom_domain}"
}

resource "cloudflare_dns_record" "dns_website_notes" {
  zone_id = data.cloudflare_zone.website.zone_id
  name    = "notes"
  type    = "CNAME"
  content = local.pages_subdomain
  ttl     = 1
  proxied = true

  depends_on = [cloudflare_pages_domain.pages_website_notes]
}

resource "cloudflare_pages_domain" "pages_website_rc" {
  account_id   = var.cloudflare_account_id
  project_name = cloudflare_pages_project.website.name
  name         = "rc.${var.custom_domain}"
}

resource "cloudflare_dns_record" "dns_website_rc" {
  zone_id = data.cloudflare_zone.website.zone_id
  name    = "rc"
  type    = "CNAME"
  content = "${local.rc_branch_slug}.${local.pages_subdomain}"
  ttl     = 1
  proxied = true

  depends_on = [cloudflare_pages_domain.pages_website_rc]
}

# Google Search Console domain verification
resource "cloudflare_dns_record" "google_verification" {
  zone_id = data.cloudflare_zone.website.zone_id
  name    = "@"
  type    = "TXT"
  content = var.google_site_verification
  ttl     = 3600
}
