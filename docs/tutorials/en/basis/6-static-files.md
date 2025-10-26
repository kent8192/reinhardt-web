# Part 6: Static Files

In this tutorial, we'll add CSS stylesheets and images to make our polls application look better.

## What are Static Files?

Static files are assets like CSS, JavaScript, images, and fonts that don't change during runtime. Reinhardt provides a comprehensive system for managing and serving these files.

## Configuring Static Files

First, add the static files dependency to `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["static"] }
```

Create a directory structure for static files:

```bash
mkdir -p static/polls/css
mkdir -p static/polls/images
```

## Adding a Stylesheet

Create `static/polls/css/style.css`:

```css
body {
  font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
  background-color: #f5f5f5;
  margin: 0;
  padding: 20px;
}

h1 {
  color: #2c3e50;
  border-bottom: 3px solid #3498db;
  padding-bottom: 10px;
}

ul {
  list-style-type: none;
  padding: 0;
}

li {
  background-color: white;
  margin: 10px 0;
  padding: 15px;
  border-radius: 5px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

li a {
  color: #3498db;
  text-decoration: none;
  font-size: 18px;
}

li a:hover {
  color: #2980b9;
  text-decoration: underline;
}

form {
  background-color: white;
  padding: 20px;
  border-radius: 5px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

input[type="radio"] {
  margin-right: 10px;
}

label {
  font-size: 16px;
  margin: 10px 0;
  display: block;
}

input[type="submit"] {
  background-color: #3498db;
  color: white;
  border: none;
  padding: 10px 20px;
  font-size: 16px;
  border-radius: 5px;
  cursor: pointer;
  margin-top: 15px;
}

input[type="submit"]:hover {
  background-color: #2980b9;
}

.no-polls {
  text-align: center;
  color: #7f8c8d;
  font-size: 18px;
  padding: 40px;
}
```

## Using Static Files in Templates

Update your templates to use the static files. Modify `templates/polls/index.html`:

```html
<!DOCTYPE html>
<html>
  <head>
    <title>Polls</title>
    <link
      rel="stylesheet"
      type="text/css"
      href="{{ 'polls/css/style.css'|static }}"
    />
  </head>
  <body>
    <h1>Latest Polls</h1>

    {% if latest_question_list %}
    <ul>
      {% for question in latest_question_list %}
      <li>
        <a href="{% url 'polls:detail' question.id %}">
          {{ question.question_text }}
        </a>
      </li>
      {% endfor %}
    </ul>
    {% else %}
    <p class="no-polls">No polls are available.</p>
    {% endif %}
  </body>
</html>
```

The `{{ 'polls/css/style.css'|static }}` template tag generates the correct URL for the static file.

## Adding Images

Let's add a background image. Download an image or create one, and save it as `static/polls/images/background.png`.

Update your CSS to use the image:

```css
body {
  font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
  background:
    linear-gradient(rgba(255, 255, 255, 0.9), rgba(255, 255, 255, 0.9)),
    url("../images/background.png");
  background-size: cover;
  background-attachment: fixed;
  margin: 0;
  padding: 20px;
}
```

**Important**: In CSS files, use relative paths (like `../images/background.png`) instead of the `static` template tag. This ensures the paths work correctly regardless of your `STATIC_URL` configuration.

## Configuring Static File Serving

Update `src/main.rs` to serve static files:

```rust
use reinhardt::prelude::*;
use reinhardt::static_files::{StaticFilesHandler, StaticFilesConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... existing setup code ...

    // Configure static files
    let static_config = StaticFilesConfig {
        static_root: "static".to_string(),
        static_url: "/static/".to_string(),
        staticfiles_dirs: vec![],
    };

    let static_handler = StaticFilesHandler::new(static_config);

    // Add static file route
    router.add_route(
        path("/static/{path:.*}", static_handler)
    );

    // ... rest of setup ...
}
```

## Static File Namespacing

Just like templates, it's a good practice to namespace your static files by putting them in a directory named after your app. This prevents naming conflicts:

```
static/
    polls/
        css/
            style.css
        images/
            background.png
            logo.png
    admin/
        css/
            admin.css
```

## Collecting Static Files for Production

In production, you'll want to collect all static files into a single directory for efficient serving. Reinhardt provides the `collectstatic` command:

```bash
# This will be available through reinhardt-admin in production
reinhardt-admin collectstatic
```

This collects all static files from your apps into a single `STATIC_ROOT` directory.

## Static File Optimization

For production, consider:

1. **File hashing**: Append hashes to filenames for cache busting
2. **Compression**: Gzip or Brotli compression for faster transfers
3. **CDN**: Serve static files from a CDN for better performance
4. **Minification**: Minify CSS and JavaScript files

Reinhardt provides built-in support for these optimizations through the `static` feature.

## Summary

In this tutorial, you learned:

- How to organize static files in your project
- How to create and use CSS stylesheets
- How to reference static files in templates using the `static` filter
- How to use relative paths for resources in CSS files
- How to configure static file serving
- Best practices for static file namespacing

Your polls app now has a clean, professional appearance!

## What's Next?

In the final tutorial, we'll explore the Reinhardt admin interface and learn how to customize it for managing poll data.

Continue to [Part 7: Admin Customization](7-admin-customization.md).