<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ title }}</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 1000px;
            margin: 50px auto;
            padding: 20px;
        }
        h1 {
            color: #333;
            border-bottom: 2px solid #007bff;
            padding-bottom: 10px;
        }
        .user-list {
            list-style-type: none;
            padding: 0;
        }
        .user-item {
            background-color: #f8f9fa;
            border: 1px solid #dee2e6;
            border-radius: 5px;
            padding: 15px;
            margin: 10px 0;
            transition: background-color 0.3s;
        }
        .user-item:hover {
            background-color: #e9ecef;
        }
        .user-name {
            font-weight: bold;
            color: #007bff;
            font-size: 1.1em;
        }
        .user-email {
            color: #6c757d;
            margin-left: 10px;
        }
        .empty-message {
            text-align: center;
            color: #999;
            font-style: italic;
            padding: 40px;
        }
    </style>
</head>
<body>
    <h1>{{ title }}</h1>

    {% if users %}
        <ul class="user-list">
        {% for user in users %}
            <li class="user-item">
                <span class="user-name">{{ user.name }}</span>
                <span class="user-email">({{ user.email }})</span>
            </li>
        {% endfor %}
        </ul>
    {% else %}
        <div class="empty-message">
            No users found.
        </div>
    {% endif %}
</body>
</html>
