# Import existing Cloudflare resources into Terraform state.
# These blocks ensure resources are always imported on every apply,
# preventing 409 Conflict errors from attempting to recreate existing resources.

# Pages project
import {
  to = cloudflare_pages_project.website
  id = "${var.cloudflare_account_id}/${var.pages_project_name}"
}

# Custom domain registration
import {
  to = cloudflare_pages_domain.website
  id = "${var.cloudflare_account_id}/${var.pages_project_name}/${var.custom_domain}"
}

# www custom domain registration
import {
  to = cloudflare_pages_domain.pages_website_www
  id = "${var.cloudflare_account_id}/${var.pages_project_name}/www.${var.custom_domain}"
}

# notes custom domain registration
import {
  to = cloudflare_pages_domain.pages_website_notes
  id = "${var.cloudflare_account_id}/${var.pages_project_name}/notes.${var.custom_domain}"
}

# rc custom domain registration
import {
  to = cloudflare_pages_domain.pages_website_rc
  id = "${var.cloudflare_account_id}/${var.pages_project_name}/rc.${var.custom_domain}"
}

# Apex domain DNS record
import {
  to = cloudflare_dns_record.website_apex
  id = "${data.cloudflare_zone.website.zone_id}/${var.dns_record_apex_id}"
}

# www subdomain DNS record
import {
  to = cloudflare_dns_record.dns_website_www
  id = "${data.cloudflare_zone.website.zone_id}/${var.dns_record_www_id}"
}

# notes subdomain DNS record
import {
  to = cloudflare_dns_record.dns_website_notes
  id = "${data.cloudflare_zone.website.zone_id}/${var.dns_record_notes_id}"
}

# rc subdomain DNS record
import {
  to = cloudflare_dns_record.dns_website_rc
  id = "${data.cloudflare_zone.website.zone_id}/${var.dns_record_rc_id}"
}

# Google Search Console verification TXT record
import {
  to = cloudflare_dns_record.google_verification
  id = "${data.cloudflare_zone.website.zone_id}/${var.dns_record_google_verification_id}"
}
