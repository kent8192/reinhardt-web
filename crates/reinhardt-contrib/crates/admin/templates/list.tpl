{% extends "base.tpl" %}

{% block title %}{{ model_verbose_name }} | {{ site_title }}{% endblock %}

{% block breadcrumbs %}
<div class="content-header">
    <div class="breadcrumbs">
        <a href="/admin/">Home</a>
        &rsaquo;
        <a href="/admin/{{ model_name|lower }}/">{{ model_verbose_name }}</a>
    </div>
    <h1>Select {{ model_verbose_name|lower }} to change</h1>
</div>
{% endblock %}

{% block extra_head %}
<style>
    /* Action bar */
    .actions {
        background: #f8f9fa;
        padding: 1rem;
        margin-bottom: 1rem;
        border-radius: 4px;
        display: flex;
        gap: 1rem;
        align-items: center;
    }

    .actions select {
        width: auto;
        min-width: 200px;
    }

    .actions button {
        white-space: nowrap;
    }

    /* Search and filters */
    .toolbar {
        margin-bottom: 1.5rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 1rem;
    }

    .search-form {
        display: flex;
        gap: 0.5rem;
        flex: 1;
        max-width: 400px;
    }

    .search-form input {
        flex: 1;
    }

    /* Filters */
    .filters {
        background: #f8f9fa;
        padding: 1rem;
        border-radius: 4px;
        margin-bottom: 1rem;
    }

    .filters h3 {
        font-size: 0.875rem;
        font-weight: 600;
        margin-bottom: 0.5rem;
        color: #666;
    }

    .filter-group {
        margin-bottom: 1rem;
    }

    .filter-group:last-child {
        margin-bottom: 0;
    }

    .filter-choices {
        list-style: none;
    }

    .filter-choices li {
        padding: 0.25rem 0;
    }

    .filter-choices a {
        color: #417690;
        text-decoration: none;
        font-size: 0.875rem;
    }

    .filter-choices a:hover {
        text-decoration: underline;
    }

    .filter-choices a.selected {
        font-weight: 600;
    }

    /* Table */
    .results {
        overflow-x: auto;
    }

    .results table {
        border: 1px solid #e0e0e0;
    }

    .results thead {
        background: #f8f9fa;
    }

    .results th {
        padding: 0.75rem;
        text-align: left;
        font-weight: 600;
        font-size: 0.875rem;
        border-bottom: 2px solid #e0e0e0;
        color: #417690;
    }

    .results td {
        padding: 0.75rem;
        border-bottom: 1px solid #f0f0f0;
        font-size: 0.875rem;
    }

    .results tbody tr:hover {
        background: #f8f9fa;
    }

    .results td a {
        color: #417690;
        text-decoration: none;
    }

    .results td a:hover {
        text-decoration: underline;
    }

    /* Pagination */
    .pagination {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-top: 1.5rem;
        padding-top: 1rem;
        border-top: 1px solid #e0e0e0;
    }

    .pagination-info {
        font-size: 0.875rem;
        color: #666;
    }

    .pagination-links {
        display: flex;
        gap: 0.5rem;
    }

    .pagination-links a,
    .pagination-links span {
        padding: 0.5rem 0.75rem;
        border: 1px solid #e0e0e0;
        border-radius: 4px;
        text-decoration: none;
        font-size: 0.875rem;
    }

    .pagination-links a {
        color: #417690;
    }

    .pagination-links a:hover {
        background: #f8f9fa;
    }

    .pagination-links span {
        color: #999;
        background: #f8f9fa;
    }

    /* Add button */
    .add-link {
        display: inline-block;
        margin-bottom: 1rem;
    }

    /* Empty state */
    .empty-state {
        text-align: center;
        padding: 3rem 1rem;
        color: #666;
    }

    .empty-state h3 {
        font-size: 1.25rem;
        margin-bottom: 0.5rem;
    }

    /* Checkbox column */
    .action-checkbox-column {
        width: 40px;
        text-align: center;
    }

    input[type="checkbox"] {
        width: auto;
        cursor: pointer;
    }
</style>
{% endblock %}

{% block content %}
<!-- Add button -->
<a href="/admin/{{ model_name|lower }}/add/" class="btn btn-primary add-link">
    Add {{ model_verbose_name|lower }}
</a>

<!-- Actions -->
{% if !actions.is_empty() %}
<form method="post" class="actions">
    <label for="action">Action:</label>
    <select name="action" id="action">
        <option value="">---------</option>
        {% for action in actions %}
        <option value="{{ action.name }}">{{ action.label }}</option>
        {% endfor %}
    </select>
    <button type="submit" class="btn btn-secondary">Go</button>
</form>
{% endif %}

<!-- Search and filters -->
<div class="toolbar">
    <!-- Search -->
    {% match search_query %}
        {% when Some with (query) %}
        <form method="get" class="search-form">
            <input type="text" name="q" value="{{ query }}" placeholder="Search...">
            <button type="submit" class="btn btn-secondary">Search</button>
        </form>
        {% when None %}
        {% endmatch %}

    <div style="flex: 1;"></div>

    <!-- Filter toggle (could be expanded) -->
    {% if !filters.is_empty() %}
    <button class="btn btn-secondary" onclick="document.querySelector('.filters').style.display = document.querySelector('.filters').style.display === 'none' ? 'block' : 'none'">
        Filters
    </button>
    {% endif %}
</div>

<!-- Filters panel -->
{% if !filters.is_empty() %}
<div class="filters" style="display: none;">
    {% for filter in filters %}
    <div class="filter-group">
        <h3>{{ filter.title }}</h3>
        <ul class="filter-choices">
            {% for choice in filter.choices %}
            <li>
                <a href="{{ choice.url }}" {% if choice.selected %}class="selected"{% endif %}>
                    {{ choice.label }}
                </a>
            </li>
            {% endfor %}
        </ul>
    </div>
    {% endfor %}
</div>
{% endif %}

<!-- Results table -->
{% if !items.is_empty() %}
<div class="results">
    <table>
        <thead>
            <tr>
                {% if !actions.is_empty() %}
                <th class="action-checkbox-column">
                    <input type="checkbox" id="action-toggle">
                </th>
                {% endif %}
                {% for field in list_display %}
                <th>
                    {{ field }}
                </th>
                {% endfor %}
            </tr>
        </thead>
        <tbody>
            {% for item in items %}
            <tr>
                {% if !actions.is_empty() %}
                <td class="action-checkbox-column">
                    {% if let Some(id_val) = item.get("id") %}
                    <input type="checkbox" name="_selected_action" value="{{ id_val }}">
                    {% endif %}
                </td>
                {% endif %}
                {% for field in list_display %}
                <td>
                    {% if let Some(val) = item.get(field) %}
                        {% if loop.first %}
                            {% if let Some(id_val) = item.get("id") %}
                            <a href="/admin/{{ model_name|lower }}/{{ id_val }}/change/">
                                {{ val }}
                            </a>
                            {% else %}
                                {{ val }}
                            {% endif %}
                        {% else %}
                            {{ val }}
                        {% endif %}
                    {% endif %}
                </td>
                {% endfor %}
            </tr>
            {% endfor %}
        </tbody>
    </table>
</div>

<!-- Pagination -->
{% if pagination.total_pages > 1 %}
<div class="pagination">
    <div class="pagination-info">
        Page {{ pagination.page }} of {{ pagination.total_pages }}
        ({{ pagination.total_count }} total)
    </div>
    <div class="pagination-links">
        {% if pagination.has_previous %}
            {% match pagination.previous_url %}
                {% when Some with (url) %}
                    <a href="{{ url }}">Previous</a>
                {% when None %}
                    <span>Previous</span>
            {% endmatch %}
        {% else %}
            <span>Previous</span>
        {% endif %}

        {% if pagination.has_next %}
            {% match pagination.next_url %}
                {% when Some with (url) %}
                    <a href="{{ url }}">Next</a>
                {% when None %}
                    <span>Next</span>
            {% endmatch %}
        {% else %}
            <span>Next</span>
        {% endif %}
    </div>
</div>
{% endif %}

{% else %}
<!-- Empty state -->
<div class="empty-state">
    <h3>No {{ model_verbose_name|lower }} found</h3>
    <p>There are no items to display.</p>
    <a href="/admin/{{ model_name|lower }}/add/" class="btn btn-primary" style="margin-top: 1rem;">
        Add the first {{ model_verbose_name|lower }}
    </a>
</div>
{% endif %}
{% endblock %}
