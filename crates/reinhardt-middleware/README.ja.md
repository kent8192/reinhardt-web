# reinhardt-middleware

Reinhardtフレームワーク向けのリクエスト/レスポンス処理パイプライン

## 概要

リクエストとレスポンスを処理するためのミドルウェアシステム。セキュリティ、パフォーマンス最適化、認証、リクエスト処理のための包括的な組み込みミドルウェアを提供します。

## 実装済み機能 ✓

### コアミドルウェアシステム

- **ミドルウェアパイプライン** - ハンドラー合成によるリクエスト/レスポンス処理チェーン
- **カスタムミドルウェアサポート** - ユーザー定義ミドルウェアの簡単な統合

### セキュリティミドルウェア

- **CORS (Cross-Origin Resource Sharing)** - プリフライトサポート付きの設定可能なCORSヘッダー
  - カスタムオリジン、メソッド、ヘッダー設定
  - 認証情報サポート
  - Max-ageキャッシング
  - 開発用の許可モード
- **CSRF保護** - `reinhardt-security`によるクロスサイトリクエストフォージェリ保護
  - トークン生成と検証
  - オリジンとリファラーチェック
  - シークレット管理とローテーション
- **コンテンツセキュリティポリシー (CSP)** - カスタマイズ可能なディレクティブによるXSS保護
  - カスタムCSPディレクティブ (default-src, script-src, style-srcなど)
  - インラインスクリプト/スタイル用のNonce生成
  - テスト用のReport-Onlyモード
  - Strictプリセット設定
- **X-Frame-Options** - クリックジャッキング保護
  - DENYモード (フレーム化なし)
  - SAMEORIGINモード (同一オリジンフレーム化のみ)
- **セキュリティヘッダー** - 包括的なHTTPセキュリティヘッダー
  - HSTS (HTTP Strict Transport Security) プリロードサポート付き
  - SSL/HTTPSリダイレクト
  - X-Content-Type-Options: nosniff
  - Referrer-Policy設定
  - Cross-Origin-Opener-Policy (COOP)
- **HTTPSリダイレクト** - HTTPからHTTPSへの自動リダイレクト
  - 設定可能な除外パス
  - カスタムステータスコード (301/302)

### パフォーマンスミドルウェア

- **GZip圧縮** - 帯域幅最適化のためのレスポンス圧縮
  - 設定可能な圧縮レベル (0-9)
  - 最小サイズしきい値
  - Content-typeフィルタリング
  - 自動Accept-Encoding検出
- **Brotli圧縮** - より良い圧縮率を持つ高度な圧縮
  - 設定可能な品質レベル (Fast, Balanced, Best)
  - ウィンドウサイズ設定 (10-24)
  - Content-typeフィルタリング
  - 自動Accept-Encoding: br検出
  - インテリジェントな圧縮 (有益な場合のみ)
- **条件付きGET** - ETagとLast-ModifiedによるHTTPキャッシング
  - 自動ETag生成 (SHA-256ベース)
  - If-None-Matchサポート
  - If-Modified-Sinceサポート
  - If-MatchとIf-Unmodified-Since検証
  - 304 Not Modified レスポンス

### 認証とリクエスト処理

- **認証** - JWTベースの認証ミドルウェア
  - Bearerトークン抽出
  - `reinhardt-auth`によるトークン検証
  - ユーザータイプサポート
- **ログ記録** - リクエスト/レスポンスのログ記録
  - タイムスタンプ、メソッド、パス、ステータスコード
  - リクエスト継続時間の追跡

### 依存性注入サポート

- **DIミドルウェア** - `reinhardt-di`との統合
  - ミドルウェアファクトリパターン
  - 注入可能なミドルウェアコンポーネント
  - 自動依存性解決

### リクエスト処理とユーティリティ

- **共通ミドルウェア** - URL正規化
  - 自動トレーリングスラッシュ追加 (append_slash)
  - WWWサブドメイン前置 (prepend_www)
  - スマートファイル拡張子検出
  - クエリパラメータの保持
- **ロケールミドルウェア** - 複数ソースからのロケール検出
  - 品質スコア付きAccept-Languageヘッダーパース
  - Cookieベースのロケール保存
  - URLパスプレフィックス検出
  - 設定可能なフォールバックロケール
- **メッセージフレームワーク** - Djangoスタイルのフラッシュメッセージ
  - セッションベースとCookieベースのストレージ
  - 複数のメッセージレベル (Debug, Info, Success, Warning, Error)
  - 一度だけのメッセージ配信
  - スレッドセーフなストレージ実装
- **リダイレクトフォールバック** - スマートな404エラーハンドリング
  - 設定可能なフォールバックURL
  - パターンベースのパスマッチング (正規表現)
  - カスタムリダイレクトステータスコード
  - リダイレクトループ防止
- **壊れたリンク検出** - 内部リンク監視
  - 内部リファラーの自動404検出
  - ドメイン正規化 (www.処理)
  - 設定可能な無視パスとユーザーエージェント
  - メール通知サポート
  - ログ記録統合
- **サイトミドルウェア** - マルチサイトサポート
  - ドメインベースのサイト検出
  - デフォルトサイトフォールバックメカニズム
  - wwwサブドメイン正規化
  - サイトIDヘッダー注入
  - スレッドセーフなサイトレジストリ
- **Flatpagesミドルウェア** - 静的ページフォールバック
  - 404インターセプトとコンテンツ置換
  - URL正規化 (トレーリングスラッシュ処理)
  - インメモリflatpageストレージ
  - テンプレートレンダリングサポート
  - 登録ベースのアクセス制御

### 可観測性とモニタリング

- **リクエストIDミドルウェア** - リクエストトレーシングと相関
  - 一意のリクエスト識別のためのUUID生成
  - リクエストチェーンを通じた自動伝播
  - カスタムヘッダー名サポート
  - X-Request-IDとX-Correlation-ID互換性
- **メトリクスミドルウェア** - Prometheus互換のメトリクス収集
  - メソッドとパスごとのリクエスト数追跡
  - パーセンタイル付きレスポンス時間ヒストグラム (p50, p95, p99)
  - ステータスコード分布
  - カスタムメトリクスサポート
  - Prometheusテキスト形式の/metricsエンドポイント
  - 設定可能な除外パス
- **トレーシングミドルウェア** - 分散トレーシングサポート
  - OpenTelemetry互換のスパン追跡
  - トレースIDとスパンIDの伝播
  - 自動スパンライフサイクル管理
  - リクエストメタデータタグ付け (メソッド、パス、ステータス)
  - 設定可能なサンプリングレート
  - エラーステータス追跡

## 関連クレート

以下のミドルウェアは別クレートで実装されています:

- **セッションミドルウェア** - `reinhardt-sessions`で実装
  - セッション管理と永続化については[reinhardt-sessions](../contrib/crates/sessions/README.md)を参照
- **キャッシュミドルウェア** - `reinhardt-cache`で実装
  - レスポンスキャッシング層については[reinhardt-cache](../utils/crates/cache/README.md)を参照
- **パーミッションミドルウェア** - `reinhardt-auth`で実装
  - ✓ パーミッションベースのアクセス制御
  - ✓ DRFスタイルのパーミッション (IsAuthenticated, IsAdminUser, IsAuthenticatedOrReadOnly)
  - ✓ モデルレベルのパーミッション (オブジェクトパーミッション)
  - ✓ パーミッション演算子 (AND, OR, NOT)
  - ✓ 高度なパーミッション (動的、条件付き、複合)
  - 詳細は[reinhardt-auth](../contrib/crates/auth/README.md)を参照
- **レート制限** - `reinhardt-rest/throttling`で実装
  - ✓ リクエストスロットリングとレート制限
  - ✓ 匿名ユーザー向けのAnonRateThrottle
  - ✓ 認証済みユーザー向けのUserRateThrottle
  - ✓ APIスコープ向けのScopedRateThrottle
  - ✓ バースト保護向けのBurstRateThrottle
  - ✓ 段階的制限向けのTieredRateThrottle
  - ✓ メモリとRedisバックエンド
  - 詳細は[reinhardt-rest/throttling](../../reinhardt-rest/crates/throttling/README.md)を参照

## CSRFミドルウェアの使用方法

### 基本的な使用方法

```rust
use reinhardt_middleware::csrf::{CsrfMiddleware, CsrfMiddlewareConfig};
use reinhardt_apps::{Handler, Middleware};
use std::sync::Arc;

// デフォルト設定
let csrf_middleware = CsrfMiddleware::new();

// 本番環境設定
let config = CsrfMiddlewareConfig::production(vec![
    "https://example.com".to_string(),
    "https://api.example.com".to_string(),
]);

let csrf_middleware = CsrfMiddleware::with_config(config);
```

### 除外パス

```rust
let config = CsrfMiddlewareConfig::default()
    .add_exempt_path("/api/webhooks".to_string())
    .add_exempt_path("/health".to_string());

let csrf_middleware = CsrfMiddleware::with_config(config);
```

### トークン抽出

CSRFトークンは以下の方法で送信できます:

1. **HTTPヘッダー** (推奨): `X-CSRFToken`ヘッダー
2. **Cookie**: `csrftoken`クッキー

```javascript
// JavaScriptからヘッダー経由でトークンを送信
fetch("/api/endpoint", {
  method: "POST",
  headers: {
    "X-CSRFToken": getCookie("csrftoken"),
    "Content-Type": "application/json",
  },
  body: JSON.stringify(data),
});
```

### 動作の仕組み

1. **GETリクエスト**: 自動的にCSRFクッキーを設定
2. **POSTリクエスト**: トークンを検証
   - ヘッダーまたはクッキーからトークンを抽出
   - Refererヘッダーをチェック (設定されている場合)
   - トークンの形式と値を検証
3. **検証失敗**: 403 Forbiddenを返す
