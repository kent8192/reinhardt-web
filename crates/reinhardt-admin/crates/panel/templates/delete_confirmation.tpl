{% extends "base.tpl" %}

{% block title %}Delete {{ model_name }} | {{ site_title }}{% endblock %}

{% block breadcrumbs %}
<div class="content-header">
    <div class="breadcrumbs">
        <a href="/admin/">Home</a>
        &rsaquo;
        <a href="/admin/{{ model_name|lower }}/">{{ model_name }}</a>
        &rsaquo;
        Delete
    </div>
    <h1>Delete {{ model_name }}</h1>
</div>
{% endblock %}

{% block extra_head %}
<style>
    /* Delete confirmation specific styles */
    .delete-confirmation {
        max-width: 800px;
    }

    .alert {
        padding: 1.5rem;
        border-radius: 4px;
        margin-bottom: 1.5rem;
    }

    .alert-danger {
        background: #f8d7da;
        border-left: 4px solid #dc3545;
        color: #721c24;
    }

    .alert h2 {
        font-size: 1.25rem;
        margin-bottom: 1rem;
        color: #721c24;
    }

    .object-name {
        font-weight: 600;
        font-size: 1.1rem;
        margin-bottom: 1rem;
        padding: 0.5rem;
        background: white;
        border-radius: 4px;
    }

    .related-objects {
        margin: 1.5rem 0;
    }

    .related-objects h3 {
        font-size: 1rem;
        margin-bottom: 1rem;
        color: #666;
    }

    .related-list {
        list-style: none;
        padding-left: 0;
    }

    .related-list li {
        padding: 0.75rem;
        background: white;
        border-radius: 4px;
        margin-bottom: 0.5rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .related-model {
        font-weight: 600;
        color: #417690;
    }

    .related-count {
        font-size: 0.875rem;
        color: #666;
        background: #f8f9fa;
        padding: 0.25rem 0.75rem;
        border-radius: 12px;
    }

    .related-items {
        list-style: none;
        padding-left: 1rem;
        margin-top: 0.5rem;
        font-size: 0.875rem;
        color: #666;
    }

    .related-items li {
        background: none;
        padding: 0.25rem 0;
        margin-bottom: 0;
    }

    .related-items li::before {
        content: "• ";
        margin-right: 0.5rem;
    }

    .total-count {
        font-size: 1.1rem;
        font-weight: 600;
        padding: 1rem;
        background: white;
        border-radius: 4px;
        margin-top: 1rem;
        text-align: center;
    }

    .total-count strong {
        color: #dc3545;
        font-size: 1.5rem;
    }

    .confirmation-actions {
        display: flex;
        gap: 1rem;
        margin-top: 2rem;
    }

    .warning-text {
        font-size: 0.875rem;
        color: #856404;
        background: #fff3cd;
        padding: 1rem;
        border-radius: 4px;
        margin-bottom: 1.5rem;
        border-left: 4px solid #ffc107;
    }
</style>
{% endblock %}

{% block content %}
<div class="delete-confirmation">
    <div class="alert alert-danger">
        <h2>Are you sure?</h2>

        <div class="object-name">
            "{{ object_repr }}"
        </div>

        <p>
            Deleting this {{ model_name|lower }} will also delete the following related objects:
        </p>
    </div>

    {% if related_objects %}
    <div class="related-objects">
        <h3>Related objects to be deleted:</h3>
        <ul class="related-list">
            {% for related in related_objects %}
            <li>
                <span class="related-model">{{ related.model_name }}</span>
                <span class="related-count">{{ related.count }} object{% if related.count != 1 %}s{% endif %}</span>
            </li>
            {% if related.items %}
            <ul class="related-items">
                {% for item in related.items %}
                    {% if loop.index <= 5 %}
                    <li>{{ item }}</li>
                    {% endif %}
                {% endfor %}
                {% if related.items | length > 5 %}
                <li><em>... and {{ related.items | length - 5 }} more</em></li>
                {% endif %}
            </ul>
            {% endif %}
            {% endfor %}
        </ul>

        <div class="total-count">
            Total: <strong>{{ total_count }}</strong> object{% if total_count != 1 %}s{% endif %} will be deleted
        </div>
    </div>
    {% endif %}

    <div class="warning-text">
        <strong>Warning:</strong> This action cannot be undone. All related data will be permanently deleted.
    </div>

    <form method="post">
        <div class="confirmation-actions">
            <button type="submit" class="btn btn-danger">
                Yes, I'm sure - delete it
            </button>
            <a href="/admin/{{ model_name|lower }}/" class="btn btn-secondary">
                No, take me back
            </a>
        </div>
    </form>
</div>
{% endblock %}
