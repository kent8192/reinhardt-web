{% extends "base.tpl" %}

{% block title %}{{ title }} | {{ site_title }}{% endblock %}

{% block breadcrumbs %}
<div class="content-header">
    <div class="breadcrumbs">
        <a href="/admin/">Home</a>
        &rsaquo;
        <a href="/admin/{{ model_name|lower }}/">{{ model_name }}</a>
        &rsaquo;
        {% if object_id %}
            <a href="/admin/{{ model_name|lower }}/{{ object_id }}/change/">{{ object_id }}</a>
        {% else %}
            Add
        {% endif %}
    </div>
    <h1>{{ title }}</h1>
</div>
{% endblock %}

{% block extra_head %}
<style>
    /* Form layout */
    .form-row {
        margin-bottom: 1.5rem;
    }

    .form-row:last-child {
        margin-bottom: 0;
    }

    .form-label {
        display: block;
        margin-bottom: 0.5rem;
        font-weight: 600;
        color: #333;
        font-size: 0.875rem;
    }

    .form-label.required::after {
        content: " *";
        color: #dc3545;
    }

    .form-help {
        display: block;
        margin-top: 0.25rem;
        font-size: 0.75rem;
        color: #666;
    }

    .form-errors {
        background: #f8d7da;
        border-left: 4px solid #dc3545;
        color: #721c24;
        padding: 0.75rem;
        margin-bottom: 1rem;
        border-radius: 4px;
    }

    .form-errors ul {
        list-style: none;
        margin: 0;
    }

    .form-errors li {
        margin-bottom: 0.25rem;
    }

    .form-errors li:last-child {
        margin-bottom: 0;
    }

    .field-error {
        display: block;
        margin-top: 0.25rem;
        font-size: 0.75rem;
        color: #dc3545;
    }

    /* Field widgets */
    .form-field {
        position: relative;
    }

    .form-field.readonly {
        opacity: 0.7;
    }

    .form-field.readonly input,
    .form-field.readonly textarea,
    .form-field.readonly select {
        background: #f8f9fa;
        cursor: not-allowed;
    }

    /* Inline formsets */
    .inline-group {
        margin-top: 2rem;
        padding: 1.5rem;
        background: #f8f9fa;
        border-radius: 4px;
    }

    .inline-group h2 {
        font-size: 1.25rem;
        margin-bottom: 1rem;
        color: #417690;
    }

    .inline-related {
        margin-bottom: 1rem;
        padding: 1rem;
        background: white;
        border-radius: 4px;
        border: 1px solid #e0e0e0;
    }

    .inline-related:last-child {
        margin-bottom: 0;
    }

    .inline-deletelink {
        display: inline-block;
        margin-top: 0.5rem;
        color: #dc3545;
        text-decoration: none;
        font-size: 0.875rem;
    }

    .inline-deletelink:hover {
        text-decoration: underline;
    }

    /* Form actions */
    .submit-row {
        display: flex;
        gap: 1rem;
        margin-top: 2rem;
        padding-top: 1.5rem;
        border-top: 1px solid #e0e0e0;
    }

    /* Multi-column layout for fields */
    .form-columns {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
        gap: 1.5rem;
    }

    /* Specific widget styles */
    textarea {
        min-height: 100px;
        resize: vertical;
    }

    select[multiple] {
        min-height: 150px;
    }

    /* Date/time widgets */
    input[type="date"],
    input[type="time"],
    input[type="datetime-local"] {
        width: auto;
        min-width: 200px;
    }

    /* Boolean fields */
    .checkbox-row {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .checkbox-row input[type="checkbox"] {
        width: auto;
    }

    .checkbox-row label {
        margin-bottom: 0;
        font-weight: normal;
    }

    /* File upload fields */
    .file-upload {
        display: flex;
        align-items: center;
        gap: 1rem;
    }

    .file-upload input[type="file"] {
        flex: 1;
    }

    .current-file {
        font-size: 0.875rem;
        color: #666;
    }

    .current-file a {
        color: #417690;
        text-decoration: none;
    }

    .current-file a:hover {
        text-decoration: underline;
    }
</style>
{% endblock %}

{% block content %}
<!-- Form errors summary -->
{% if errors %}
<div class="form-errors">
    <strong>Please correct the following errors:</strong>
    <ul>
        {% for error in errors %}
        <li>{{ error }}</li>
        {% endfor %}
    </ul>
</div>
{% endif %}

<!-- Main form -->
<form method="post" enctype="multipart/form-data">
    <!-- Form fields -->
    <div class="form-fields">
        {% for field in fields %}
        <div class="form-row">
            <div class="form-field {% if field.is_readonly %}readonly{% endif %}">
                <label class="form-label {% if field.is_required %}required{% endif %}" for="id_{{ field.name }}">
                    {{ field.label }}
                </label>

                <!-- Widget HTML (pre-rendered) -->
                <div class="field-widget">
                    {{ field.widget_html|safe }}
                </div>

                <!-- Help text -->
                {% if field.help_text %}
                    <span class="form-help">{{ field.help_text }}</span>
                {% endif %}

                <!-- Field errors -->
                {% if field.errors %}
                    {% for error in field.errors %}
                    <span class="field-error">{{ error }}</span>
                    {% endfor %}
                {% endif %}
            </div>
        </div>
        {% endfor %}
    </div>

    <!-- Inline formsets -->
    {% if inlines %}
        {% for inline in inlines %}
        <div class="inline-group">
            <h2>{{ inline.verbose_name }}</h2>

            {% for form in inline.forms %}
            <div class="inline-related">
                {% for field in form.fields %}
                <div class="form-row">
                    <div class="form-field {% if field.is_readonly %}readonly{% endif %}">
                        <label class="form-label" for="id_{{ inline.model_name }}_{{ loop.index0 }}_{{ field.name }}">
                            {{ field.label }}
                        </label>

                        <div class="field-widget">
                            {{ field.widget_html|safe }}
                        </div>

                        {% if field.help_text %}
                            <span class="form-help">{{ field.help_text }}</span>
                        {% endif %}

                        {% if field.errors %}
                            {% for error in field.errors %}
                            <span class="field-error">{{ error }}</span>
                            {% endfor %}
                        {% endif %}
                    </div>
                </div>
                {% endfor %}

                <a href="#" class="inline-deletelink">Delete</a>
            </div>
            {% endfor %}

            <button type="button" class="btn btn-secondary" onclick="alert('Add another item')">
                Add another {{ inline.verbose_name|lower }}
            </button>
        </div>
        {% endfor %}
    {% endif %}

    <!-- Submit buttons -->
    <div class="submit-row">
        <button type="submit" name="_save" class="btn btn-primary">
            Save
        </button>
        <button type="submit" name="_continue" class="btn btn-secondary">
            Save and continue editing
        </button>
        <button type="submit" name="_addanother" class="btn btn-secondary">
            Save and add another
        </button>
        <a href="/admin/{{ model_name|lower }}/" class="btn btn-secondary">
            Cancel
        </a>
        {% if object_id %}
            <a href="/admin/{{ model_name|lower }}/{{ object_id }}/delete/" class="btn btn-danger" style="margin-left: auto;">
                Delete
            </a>
        {% endif %}
    </div>
</form>
{% endblock %}
