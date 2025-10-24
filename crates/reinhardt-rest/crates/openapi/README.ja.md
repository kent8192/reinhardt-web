# reinhardt-openapi

OpenAPIスキーマ生成とSwagger UI統合

## 概要

APIエンドポイント、シリアライザー、ビューセットからのOpenAPI 3.0スキーマの自動生成です。リクエスト/レスポンススキーマ、認証要件、パラメータ説明を含む完全なAPIドキュメントを生成します。`utoipa-swagger-ui`を使用したSwagger UI統合も含まれています。

## 機能

- **OpenAPI 3.0**: 完全なOpenAPI 3.0仕様サポート
- **自動生成**: ViewSetからの自動スキーマ生成
- **Swagger UI**: `utoipa-swagger-ui`を使用したSwagger UI統合
- **カスタマイズ**: 生成されたスキーマのオーバーライドと拡張
- **YAML/JSON**: 両形式でのスキーマエクスポート
- **Redocサポート**: Redocによる代替ドキュメントUI

## 使用方法

## 基本的なスキーマ生成

```rustuse reinhardt_openapi::{SchemaGenerator, OpenApiSchema};

// ViewSetからスキーマを生成let generator = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation");

let schema = generator.generate()?;let json = schema.to_json()?;
```

## Swagger UI統合

```rustuse reinhardt_openapi::{SchemaGenerator, SwaggerUI};
use reinhardt_apps::{Request, Response};

// スキーマを作成let schema = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation")
    .generate()?;

// Swagger UIハンドラーを作成let swagger_ui = SwaggerUI::new(schema);

// リクエストを処理async fn handle_swagger_request(request: Request) -> Result<Response> {
    swagger_ui.handle(request).await
}
```

## Redoc UI統合

```rustuse reinhardt_openapi::{SchemaGenerator, RedocUI};

// スキーマを作成let schema = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation")
    .generate()?;

// Redoc UIハンドラーを作成let redoc_ui = RedocUI::new(schema);

// HTMLを生成let html = redoc_ui.render_html()?;
```

## APIエンドポイント

SwaggerUIを使用する場合、以下のエンドポイントが自動的に利用可能になります：

- `/swagger-ui/` - Swagger UI HTMLインターフェース
- `/swagger-ui/swagger-ui-init.js` - Swagger UI初期化スクリプト
- `/swagger-ui/swagger-ui.css` - Swagger UIスタイル
- `/swagger-ui/swagger-ui-bundle.js` - Swagger UI JavaScriptバンドル
- `/api-docs/openapi.json` - JSON形式のOpenAPI仕様

## 以前のバージョンからの移行

このバージョンでは、カスタムテンプレートの代わりに`utoipa-swagger-ui`を使用しています。APIは大部分互換性を保っていますが、内部実装の詳細が変更されています：

- テンプレートは使用されなくなりました（askama依存関係を削除）
- Swagger UIアセットは`utoipa-swagger-ui`から直接提供されます
- OpenAPIスキーマは内部的に`utoipa`形式に変換されます
- 既存のすべてのパブリックAPIは変更されていません