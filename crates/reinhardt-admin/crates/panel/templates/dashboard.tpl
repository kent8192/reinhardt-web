{% extends "base.tpl" %}

{% block title %}Dashboard | {{ site_title }}{% endblock %}

{% block breadcrumbs %}
<div class="content-header">
    <h1>Dashboard</h1>
    <p style="color: #666; margin-top: 0.5rem;">Welcome to the Reinhardt administration interface</p>
</div>
{% endblock %}

{% block extra_head %}
<style>
    /* Dashboard layout */
    .dashboard {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 1.5rem;
    }

    /* Widget styles */
    .widget {
        background: white;
        border-radius: 8px;
        box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        overflow: hidden;
        transition: box-shadow 0.2s;
    }

    .widget:hover {
        box-shadow: 0 4px 8px rgba(0,0,0,0.15);
    }

    .widget-header {
        background: #417690;
        color: white;
        padding: 1rem 1.5rem;
        font-weight: 600;
        font-size: 1rem;
    }

    .widget-body {
        padding: 1.5rem;
    }

    .widget-footer {
        padding: 1rem 1.5rem;
        background: #f8f9fa;
        border-top: 1px solid #e0e0e0;
    }

    /* Stat widget */
    .stat-widget .stat-value {
        font-size: 2.5rem;
        font-weight: 700;
        color: #417690;
        margin-bottom: 0.5rem;
    }

    .stat-widget .stat-label {
        font-size: 0.875rem;
        color: #666;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    /* App list widget */
    .app-list {
        list-style: none;
    }

    .app-item {
        padding: 0.75rem 0;
        border-bottom: 1px solid #f0f0f0;
    }

    .app-item:last-child {
        border-bottom: none;
    }

    .app-name {
        font-weight: 600;
        color: #417690;
        text-decoration: none;
        font-size: 1rem;
        display: block;
        margin-bottom: 0.5rem;
    }

    .app-name:hover {
        color: #2b5064;
    }

    .model-list {
        list-style: none;
        padding-left: 1rem;
    }

    .model-item {
        padding: 0.25rem 0;
        display: flex;
        gap: 0.5rem;
    }

    .model-item a {
        color: #666;
        text-decoration: none;
        font-size: 0.875rem;
    }

    .model-item a:hover {
        color: #417690;
        text-decoration: underline;
    }

    /* Recent actions */
    .recent-actions {
        list-style: none;
    }

    .action-item {
        padding: 0.75rem 0;
        border-bottom: 1px solid #f0f0f0;
    }

    .action-item:last-child {
        border-bottom: none;
    }

    .action-header {
        display: flex;
        justify-content: space-between;
        align-items: start;
        margin-bottom: 0.25rem;
    }

    .action-type {
        font-weight: 600;
        font-size: 0.875rem;
    }

    .action-type.added {
        color: #28a745;
    }

    .action-type.changed {
        color: #ffc107;
    }

    .action-type.deleted {
        color: #dc3545;
    }

    .action-time {
        font-size: 0.75rem;
        color: #999;
    }

    .action-details {
        font-size: 0.875rem;
        color: #666;
    }

    .action-object {
        color: #417690;
        text-decoration: none;
    }

    .action-object:hover {
        text-decoration: underline;
    }

    .action-user {
        font-size: 0.75rem;
        color: #999;
        margin-top: 0.25rem;
    }

    /* Quick links */
    .quick-links {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
        gap: 0.75rem;
    }

    .quick-link {
        display: block;
        padding: 1rem;
        background: #f8f9fa;
        border-radius: 4px;
        text-align: center;
        text-decoration: none;
        color: #417690;
        font-weight: 500;
        font-size: 0.875rem;
        transition: all 0.2s;
    }

    .quick-link:hover {
        background: #417690;
        color: white;
    }

    /* Empty state */
    .empty-state {
        text-align: center;
        padding: 2rem;
        color: #999;
    }
</style>
{% endblock %}

{% block content %}
<div class="dashboard">
    <!-- Widgets -->
    {% for widget in widgets %}
    <div class="widget {{ widget.css_class }}">
        <div class="widget-header">
            {{ widget.title }}
        </div>
        <div class="widget-body">
            {{ widget.content_html|safe }}
        </div>
    </div>
    {% endfor %}

    <!-- Applications list -->
    {% if available_apps %}
    <div class="widget" style="grid-column: 1 / -1;">
        <div class="widget-header">
            Applications
        </div>
        <div class="widget-body">
            <ul class="app-list">
                {% for app in available_apps %}
                <li class="app-item">
                    <a href="/admin/{{ app.label }}/" class="app-name">
                        {{ app.name }}
                    </a>
                    {% if app.models %}
                    <ul class="model-list">
                        {% for model in app.models %}
                        <li class="model-item">
                            <a href="{{ model.url }}">{{ model.label }}</a>
                            {% if model.add_url %}
                                <span style="color: #ddd;">|</span>
                                <a href="{{ model.add_url }}">Add</a>
                            {% endif %}
                        </li>
                        {% endfor %}
                    </ul>
                    {% endif %}
                </li>
                {% endfor %}
            </ul>
        </div>
    </div>
    {% endif %}

    <!-- Recent actions -->
    {% if recent_actions %}
    <div class="widget">
        <div class="widget-header">
            Recent Actions
        </div>
        <div class="widget-body">
            <ul class="recent-actions">
                {% for action in recent_actions %}
                <li class="action-item">
                    <div class="action-header">
                        <span class="action-type {{ action.action|lower }}">
                            {{ action.action }}
                        </span>
                        <span class="action-time">
                            {{ action.timestamp }}
                        </span>
                    </div>
                    <div class="action-details">
                        <a href="#" class="action-object">{{ action.object_repr }}</a>
                        <span style="color: #999;">in</span>
                        {{ action.model_name }}
                    </div>
                    <div class="action-user">
                        by {{ action.user }}
                    </div>
                </li>
                {% endfor %}
            </ul>
        </div>
        <div class="widget-footer">
            <a href="/admin/logs/" style="color: #417690; text-decoration: none; font-size: 0.875rem;">
                View all logs →
            </a>
        </div>
    </div>
    {% endif %}

    <!-- Quick links widget (example) -->
    <div class="widget">
        <div class="widget-header">
            Quick Links
        </div>
        <div class="widget-body">
            <div class="quick-links">
                <a href="/admin/users/" class="quick-link">Users</a>
                <a href="/admin/settings/" class="quick-link">Settings</a>
                <a href="/docs/" class="quick-link">Documentation</a>
                <a href="/api/" class="quick-link">API</a>
            </div>
        </div>
    </div>
</div>

<!-- Empty state if no widgets -->
{% if not widgets and not available_apps %}
<div class="empty-state">
    <h3>Welcome to Reinhardt Admin</h3>
    <p>No applications registered yet.</p>
</div>
{% endif %}
{% endblock %}
