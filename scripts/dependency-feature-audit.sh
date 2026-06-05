#!/usr/bin/env bash
set -euo pipefail

label="${1:-baseline}"
root="$(git rev-parse --show-toplevel)"
out_dir="/tmp/reinhardt-dependency-feature-audit/${label}"
mkdir -p "${out_dir}"

heavy_pattern='(^| )((sqlx|sqlx-core|sqlx-mysql|sqlx-postgres|sqlx-sqlite|utoipa|utoipa-gen|utoipa-swagger-ui|tera|lettre|argon2|jsonwebtoken|image|image-webp|aws-config|aws-sdk-[^ ]+|mongodb|redis|wasm-bindgen|web-sys|quick-xml|multer|rskafka|tonic|tonic-prost-build|notify|oxc_[^ ]+|oxipng|webp) v)'

combos=(
  "facade-no-default|reinhardt-web|--no-default-features"
  "facade-core|reinhardt-web|--no-default-features --features core"
  "facade-minimal|reinhardt-web|--no-default-features --features minimal"
  "facade-default|reinhardt-web|"
  "facade-standard|reinhardt-web|--no-default-features --features standard"
  "facade-full|reinhardt-web|--no-default-features --features full"
  "facade-database|reinhardt-web|--no-default-features --features database"
  "facade-db-postgres|reinhardt-web|--no-default-features --features db-postgres"
  "facade-db-sqlite|reinhardt-web|--no-default-features --features db-sqlite"
  "facade-rest|reinhardt-web|--no-default-features --features rest"
  "facade-openapi|reinhardt-web|--no-default-features --features openapi"
  "facade-auth|reinhardt-web|--no-default-features --features auth"
  "facade-auth-jwt|reinhardt-web|--no-default-features --features auth-jwt"
  "facade-pages|reinhardt-web|--no-default-features --features pages"
  "core-minimal|reinhardt-core|--no-default-features"
  "core-full|reinhardt-core|--no-default-features --features core-full"
  "utils-minimal|reinhardt-utils|--no-default-features"
  "utils-full|reinhardt-utils|--no-default-features --features utils-full"
  "rest-minimal|reinhardt-rest|--no-default-features"
  "rest-full|reinhardt-rest|--no-default-features --features rest-full"
  "auth-minimal|reinhardt-auth|--no-default-features"
  "auth-full|reinhardt-auth|--no-default-features --features auth-full"
  "db-minimal|reinhardt-db|--no-default-features"
  "db-full|reinhardt-db|--no-default-features --features database-full"
)

summary="${out_dir}/summary.tsv"
printf "combo\tpackage\tcrate_count\theavy_count\theavy_crates\n" > "${summary}"

cd "${root}"

declare -A max_crates=(
  ["facade-no-default"]=200
  ["facade-core"]=320
  ["facade-minimal"]=310
  ["facade-default"]=610
  ["facade-standard"]=610
  ["facade-database"]=470
  ["facade-db-postgres"]=470
  ["facade-db-sqlite"]=470
  ["facade-pages"]=470
)
failures=()

for combo in "${combos[@]}"; do
  IFS='|' read -r name pkg flags <<< "${combo}"
  tree_file="${out_dir}/${name}.tree"
  heavy_file="${out_dir}/${name}.heavy"

  echo "=== Auditing ${name} (${pkg} ${flags}) ==="
  # shellcheck disable=SC2086
  cargo tree -p "${pkg}" ${flags} -e normal --prefix none > "${tree_file}"

  if sort -u "${tree_file}" | rg "${heavy_pattern}" > "${heavy_file}"; then
    :
  else
    status=$?
    if [[ "${status}" -ne 1 ]]; then
      echo "heavy dependency scan failed for ${name} (rg exit ${status})" >&2
      exit "${status}"
    fi
  fi
  crate_count="$(sort -u "${tree_file}" | wc -l | tr -d ' ')"
  heavy_count="$(wc -l < "${heavy_file}" | tr -d ' ')"
  heavy_crates="$(cut -d' ' -f1 "${heavy_file}" | sort -u | paste -sd ',' -)"

  printf "%s\t%s\t%s\t%s\t%s\n" \
    "${name}" "${pkg}" "${crate_count}" "${heavy_count}" "${heavy_crates}" >> "${summary}"

  if [[ -n "${max_crates[$name]:-}" ]] && (( crate_count > max_crates[$name] )); then
    failures+=("${name}: crate_count ${crate_count} exceeds max ${max_crates[$name]}")
  fi
done

echo "Summary: ${summary}"

if (( ${#failures[@]} > 0 )); then
  printf "Dependency feature audit failed:\n" >&2
  printf "  - %s\n" "${failures[@]}" >&2
  exit 1
fi
