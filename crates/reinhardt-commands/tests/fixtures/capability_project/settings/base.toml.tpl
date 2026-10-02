staticfiles_dirs = ["assets"]

[core]
secret_key = "${REINHARDT_CAPABILITY_RUNTIME_SECRET_6336}"
installed_apps = []

[core.databases.default]
engine = "postgresql"
name = "runtime"
host = "127.0.0.1"
port = 9
password = "${REINHARDT_CAPABILITY_DB_PASSWORD_6336}"

[static]
url = "/static/"
root = "dist"

[cloud]
jwt_secret = "${REINHARDT_CAPABILITY_JWT_SECRET_6336}"

[sessions]
secret = "${REINHARDT_CAPABILITY_SESSION_SECRET_6336}"
