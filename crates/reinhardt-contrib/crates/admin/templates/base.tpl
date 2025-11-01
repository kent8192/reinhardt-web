<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}{{ site_title }}{% endblock %}</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background: #f5f5f5;
            color: #333;
            line-height: 1.6;
        }

        /* Header */
        .header {
            background: #417690;
            color: white;
            padding: 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }

        .header-inner {
            max-width: 1200px;
            margin: 0 auto;
            padding: 1rem 2rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .site-name {
            font-size: 1.5rem;
            font-weight: 600;
            color: white;
            text-decoration: none;
        }

        .site-name:hover {
            color: #ffc;
        }

        .user-tools {
            display: flex;
            gap: 1rem;
            align-items: center;
        }

        .user-tools a {
            color: white;
            text-decoration: none;
            font-size: 0.875rem;
        }

        .user-tools a:hover {
            color: #ffc;
        }

        /* Navigation */
        .nav {
            background: #2b5064;
            padding: 0;
        }

        .nav-inner {
            max-width: 1200px;
            margin: 0 auto;
            padding: 0 2rem;
            display: flex;
            gap: 2rem;
        }

        .nav a {
            color: white;
            text-decoration: none;
            padding: 0.75rem 0;
            display: block;
            font-size: 0.875rem;
            border-bottom: 3px solid transparent;
            transition: border-color 0.2s;
        }

        .nav a:hover,
        .nav a.active {
            border-bottom-color: #ffc;
        }

        /* Main Content */
        .container {
            max-width: 1200px;
            margin: 2rem auto;
            padding: 0 2rem;
        }

        /* Content header */
        .content-header {
            margin-bottom: 2rem;
        }

        .content-header h1 {
            font-size: 2rem;
            font-weight: 600;
            color: #417690;
            margin-bottom: 0.5rem;
        }

        .breadcrumbs {
            font-size: 0.875rem;
            color: #666;
        }

        .breadcrumbs a {
            color: #417690;
            text-decoration: none;
        }

        .breadcrumbs a:hover {
            text-decoration: underline;
        }

        /* Messages */
        .messages {
            list-style: none;
            margin-bottom: 1.5rem;
        }

        .message {
            padding: 1rem;
            margin-bottom: 0.5rem;
            border-radius: 4px;
            background: #d4edda;
            border-left: 4px solid #28a745;
            color: #155724;
        }

        .message.error {
            background: #f8d7da;
            border-left-color: #dc3545;
            color: #721c24;
        }

        .message.warning {
            background: #fff3cd;
            border-left-color: #ffc107;
            color: #856404;
        }

        /* Main content area */
        .content {
            background: white;
            border-radius: 8px;
            padding: 2rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }

        /* Footer */
        .footer {
            text-align: center;
            padding: 2rem;
            color: #666;
            font-size: 0.875rem;
            margin-top: 3rem;
        }

        /* Buttons */
        .btn {
            display: inline-block;
            padding: 0.5rem 1rem;
            border-radius: 4px;
            text-decoration: none;
            font-size: 0.875rem;
            font-weight: 500;
            border: none;
            cursor: pointer;
            transition: all 0.2s;
        }

        .btn-primary {
            background: #417690;
            color: white;
        }

        .btn-primary:hover {
            background: #2b5064;
        }

        .btn-secondary {
            background: #6c757d;
            color: white;
        }

        .btn-secondary:hover {
            background: #5a6268;
        }

        .btn-danger {
            background: #dc3545;
            color: white;
        }

        .btn-danger:hover {
            background: #c82333;
        }

        /* Table base styles */
        table {
            width: 100%;
            border-collapse: collapse;
        }

        /* Forms */
        .form-group {
            margin-bottom: 1.5rem;
        }

        label {
            display: block;
            margin-bottom: 0.5rem;
            font-weight: 500;
            color: #333;
        }

        input[type="text"],
        input[type="email"],
        input[type="password"],
        input[type="number"],
        textarea,
        select {
            width: 100%;
            padding: 0.5rem;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-size: 0.875rem;
        }

        input:focus,
        textarea:focus,
        select:focus {
            outline: none;
            border-color: #417690;
            box-shadow: 0 0 0 3px rgba(65, 118, 144, 0.1);
        }

        .help-text {
            font-size: 0.75rem;
            color: #666;
            margin-top: 0.25rem;
        }

        .error-text {
            font-size: 0.75rem;
            color: #dc3545;
            margin-top: 0.25rem;
        }

        /* Responsive */
        @media (max-width: 768px) {
            .header-inner,
            .nav-inner,
            .container {
                padding-left: 1rem;
                padding-right: 1rem;
            }

            .nav-inner {
                flex-direction: column;
                gap: 0;
            }

            .content {
                padding: 1rem;
            }
        }
    </style>
    {% block extra_head %}{% endblock %}
</head>
<body>
    <!-- Header -->
    <header class="header">
        <div class="header-inner">
            <a href="/admin/" class="site-name">{{ site_header }}</a>
            <div class="user-tools">
                {% if user %}
                    <span>Welcome, {{ user.username }}</span>
                    {% if user.is_superuser %}
                        <span class="badge">Superuser</span>
                    {% endif %}
                    <a href="/admin/logout/">Log out</a>
                {% else %}
                    <a href="/admin/login/">Log in</a>
                {% endif %}
            </div>
        </div>
    </header>

    <!-- Navigation -->
    {% if available_apps %}
    <nav class="nav">
        <div class="nav-inner">
            <a href="/admin/">Home</a>
            {% for app in available_apps %}
                <a href="/admin/{{ app.label }}/">{{ app.name }}</a>
            {% endfor %}
        </div>
    </nav>
    {% endif %}

    <!-- Main Content -->
    <main class="container">
        {% block breadcrumbs %}{% endblock %}

        <div class="content">
            {% block content %}{% endblock %}
        </div>
    </main>

    <!-- Footer -->
    <footer class="footer">
        <p>Powered by Reinhardt Admin</p>
    </footer>

    {% block extra_js %}{% endblock %}
</body>
</html>
