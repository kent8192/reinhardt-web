# パート6: 静的ファイル

このチュートリアルでは、投票アプリケーションの見た目を良くするためにCSSスタイルシートと画像を追加します。

## 静的ファイルとは？

静的ファイルは、CSS、JavaScript、画像、フォントなど、実行時に変更されないアセットです。Reinhardtはこれらのファイルを管理・配信するための包括的なシステムを提供します。

## 静的ファイルの設定

まず、`Cargo.toml`に静的ファイルの依存関係を追加します：

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["static"] }
```

静的ファイル用のディレクトリ構造を作成します：

```bash
mkdir -p static/polls/css
mkdir -p static/polls/images
```

## スタイルシートの追加

`static/polls/css/style.css`を作成します：

```css
body {
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
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
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
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
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
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

## テンプレートで静的ファイルを使用

静的ファイルを使用するようにテンプレートを更新します。`templates/polls/index.html`を変更します：

```html
<!DOCTYPE html>
<html>
<head>
    <title>投票</title>
    <link rel="stylesheet" type="text/css" href="{{ 'polls/css/style.css'|static }}">
</head>
<body>
    <h1>最新の投票</h1>

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
        <p class="no-polls">利用可能な投票はありません。</p>
    {% endif %}
</body>
</html>
```

`{{ 'polls/css/style.css'|static }}`テンプレートタグは、静的ファイルの正しいURLを生成します。

## 画像の追加

背景画像を追加しましょう。画像をダウンロードまたは作成し、`static/polls/images/background.png`として保存します。

画像を使用するようにCSSを更新します：

```css
body {
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    background: linear-gradient(rgba(255,255,255,0.9), rgba(255,255,255,0.9)),
                url('../images/background.png');
    background-size: cover;
    background-attachment: fixed;
    margin: 0;
    padding: 20px;
}
```

**重要**: CSSファイル内では、`static`テンプレートタグではなく、相対パス（`../images/background.png`など）を使用します。これにより、`STATIC_URL`設定に関係なくパスが正しく機能します。

## 静的ファイル配信の設定

`src/main.rs`を更新して静的ファイルを配信します：

```rust
use reinhardt::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... 既存のセットアップコード ...

    // 静的ファイルを設定
    let static_config = StaticFilesConfig {
        static_root: "static".to_string(),
        static_url: "/static/".to_string(),
        staticfiles_dirs: vec![],
    };

    let static_handler = StaticFilesHandler::new(static_config);

    // 静的ファイルルートを追加
    router.add_route(
        path("/static/{path:.*}", static_handler)
    );

    // ... 残りのセットアップ ...
}
```

## 静的ファイルの名前空間化

テンプレートと同様に、アプリ名を付けたディレクトリに静的ファイルを配置して名前空間化することがベストプラクティスです。これにより名前の競合を防ぎます：

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

## 本番環境用の静的ファイル収集

本番環境では、効率的な配信のためにすべての静的ファイルを単一のディレクトリに収集します。Reinhardtは`collectstatic`コマンドを提供します：

```bash
# 本番環境ではreinhardt-adminを通じて利用可能
reinhardt-admin collectstatic
```

これにより、アプリのすべての静的ファイルが単一の`STATIC_ROOT`ディレクトリに収集されます。

## 静的ファイルの最適化

本番環境では、以下を検討してください：

1. **ファイルハッシュ**: キャッシュバスティングのためにファイル名にハッシュを追加
2. **圧縮**: より高速な転送のためにGzipまたはBrotli圧縮
3. **CDN**: より良いパフォーマンスのためにCDNから静的ファイルを配信
4. **ミニファイ**: CSSとJavaScriptファイルをミニファイ

Reinhardtは静的ファイル機能を通じてこれらの最適化の組み込みサポートを提供します。

## まとめ

このチュートリアルで学んだこと：

- プロジェクト内の静的ファイルの整理方法
- CSSスタイルシートの作成と使用方法
- `static`フィルタを使用してテンプレート内で静的ファイルを参照する方法
- CSSファイル内のリソースに相対パスを使用する方法
- 静的ファイル配信の設定方法
- 静的ファイルの名前空間化のベストプラクティス

投票アプリがクリーンでプロフェッショナルな外観になりました！

## 次は何をする？

最後のチュートリアルでは、Reinhardt管理画面を探索し、投票データを管理するためにカスタマイズする方法を学びます。

[パート7: 管理画面のカスタマイズ](7-admin-customization.md)に進んでください。
