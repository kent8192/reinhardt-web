[build]
target = "index.html"
public-url = "/"
dist = "dist"
filehash = false

[watch]
watch = ["src/", "index.html"]
ignore = ["target/"]

[serve]
address = "127.0.0.1"
port = 8080
open = false

[clean]
dist = "dist"
cargo = true
