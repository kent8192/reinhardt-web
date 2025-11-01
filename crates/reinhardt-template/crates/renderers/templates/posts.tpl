<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Posts ({{ total }})</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 1200px;
            margin: 50px auto;
            padding: 20px;
            background-color: #f5f5f5;
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #007bff;
            padding-bottom: 15px;
            margin-bottom: 30px;
        }
        .post-count {
            font-size: 0.8em;
            color: #666;
            font-weight: normal;
        }
        .post-list {
            list-style-type: none;
            padding: 0;
        }
        .post-item {
            background-color: white;
            border: 1px solid #dee2e6;
            border-radius: 8px;
            padding: 25px;
            margin: 20px 0;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            transition: box-shadow 0.3s;
        }
        .post-item:hover {
            box-shadow: 0 4px 8px rgba(0,0,0,0.15);
        }
        .post-title {
            font-size: 1.5em;
            color: #007bff;
            margin: 0 0 15px 0;
        }
        .post-content {
            color: #555;
            line-height: 1.6;
            margin: 15px 0;
        }
        .post-meta {
            color: #999;
            font-size: 0.9em;
            margin-top: 15px;
            padding-top: 15px;
            border-top: 1px solid #eee;
        }
        .post-author {
            font-weight: bold;
            color: #666;
        }
        .empty-message {
            text-align: center;
            color: #999;
            font-style: italic;
            padding: 60px;
            background-color: white;
            border-radius: 8px;
        }
    </style>
</head>
<body>
    <h1>All Posts <span class="post-count">({{ total }} post{% if total != 1 %}s{% endif %})</span></h1>

    {% if posts %}
        <ul class="post-list">
        {% for post in posts %}
            <li class="post-item">
                <h2 class="post-title">{{ post.title }}</h2>
                <div class="post-content">{{ post.content }}</div>
                <div class="post-meta">
                    <span class="post-author">by {{ post.author }}</span>
                </div>
            </li>
        {% endfor %}
        </ul>
    {% else %}
        <div class="empty-message">
            No posts available yet. Be the first to create one!
        </div>
    {% endif %}
</body>
</html>
