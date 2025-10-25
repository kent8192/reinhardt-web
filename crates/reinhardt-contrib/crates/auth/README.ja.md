# reinhardt-auth

Reinhardtフレームワークのための認証・認可システム

## 概要

DjangoとDjango REST Frameworkにインスパイアされた包括的な認証・認可システム。JWTトークン、パーミッションクラス、ユーザーモデル、Argon2によるパスワードハッシュを提供します。

## 実装済み ✓

### コア認証

#### JWT (JSON Web Token) 認証

- **クレーム管理**: ユーザー識別、有効期限、発行時刻を持つ`Claims`構造体
- **トークン生成**: デフォルトで24時間の有効期限を自動設定
- **トークン検証**: 有効期限チェックと署名検証機能を内蔵
- **エンコード/デコード**: 完全なJWTトークンのエンコードとデコード対応

```rust
use reinhardt_auth::jwt::{JwtAuth, Claims};
use chrono::Duration;

let jwt_auth = JwtAuth::new(b"my-secret-key");
let token = jwt_auth.generate_token("user123".to_string(), "john_doe".to_string()).unwrap();
let claims = jwt_auth.verify_token(&token).unwrap();
```

#### HTTP Basic認証

- **BasicAuthentication**: ユーザー管理機能を持つHTTP Basic認証バックエンド
- **Base64エンコード/デコード**: 標準のHTTP Basic認証ヘッダーパース
- **ユーザー登録**: ユーザー名/パスワードのペアでユーザーを追加
- **リクエスト認証**: Authorizationヘッダーから認証情報を抽出・検証

```rust
use reinhardt_auth::{HttpBasicAuth, AuthenticationBackend};

let mut auth = HttpBasicAuth::new();
auth.add_user("alice", "secret123");

// Basic認証ヘッダー付きリクエストが認証される
let result = auth.authenticate(&request).unwrap();
```

### ユーザー管理

#### Userトレイト

- **コアユーザーインターフェース**: 認証済みユーザーと匿名ユーザーの統一トレイト
- **ユーザー識別**: `id()`, `username()`, `get_username()`メソッド
- **認証ステータス**: `is_authenticated()`, `is_active()`, `is_admin()`チェック
- **Django互換性**: Djangoのユーザーインターフェースと互換性のあるメソッド

#### User実装

- **SimpleUser**: UUID、ユーザー名、メール、アクティブ/管理者フラグを持つフル機能ユーザー
- **AnonymousUser**: 未認証の訪問者を表すゼロサイズ型
- **シリアライゼーション対応**: SimpleUserのSerde統合

```rust
use reinhardt_auth::{User, SimpleUser, AnonymousUser};
use uuid::Uuid;

let user = SimpleUser {
    id: Uuid::new_v4(),
    username: "john".to_string(),
    email: "john@example.com".to_string(),
    is_active: true,
    is_admin: false,
};

assert!(user.is_authenticated());
assert!(!user.is_admin());
```

### パスワードセキュリティ

#### パスワードハッシュ

- **PasswordHasherトレイト**: 組み合わせ可能なパスワードハッシュインターフェース
- **Argon2Hasher**: 本番環境対応のArgon2id実装（推奨）
- **ハッシュ生成**: OSの乱数生成器を使用したセキュアなソルト生成
- **パスワード検証**: セキュリティのための定数時間比較

```rust
use reinhardt_auth::{Argon2Hasher, PasswordHasher};

let hasher = Argon2Hasher::new();
let hash = hasher.hash("my_password").unwrap();
assert!(hasher.verify("my_password", &hash).unwrap());
```

### 認証バックエンド

#### AuthBackendトレイト

- **組み合わせ可能なアーキテクチャ**: 複数の認証戦略をサポート
- **非同期対応**: `async_trait`による完全なasync/await統合
- **ユーザー認証**: `authenticate(username, password)`メソッド
- **ユーザールックアップ**: セッション復元のための`get_user(user_id)`

#### コンポジット認証

- **CompositeAuthBackend**: 複数の認証バックエンドを連鎖
- **フォールバック対応**: 成功するまでバックエンドを順番に試行
- **柔軟な設定**: 実行時にバックエンドを動的に追加

```rust
use reinhardt_auth::CompositeAuthBackend;

let mut composite = CompositeAuthBackend::new();
composite.add_backend(Box::new(database_backend));
composite.add_backend(Box::new(ldap_backend));

// 最初にデータベース、次にLDAPを試行
let user = composite.authenticate("alice", "password").await;
```

### パーミッションシステム

#### Permissionトレイト

- **パーミッションインターフェース**: コンテキストを持つ非同期`has_permission()`メソッド
- **PermissionContext**: 認証フラグを持つリクエスト対応コンテキスト
- **組み合わせ可能なパーミッション**: 複雑なパーミッションロジックを構築

#### 組み込みパーミッションクラス

- **AllowAny**: 認証なしですべてのリクエストを許可
- **IsAuthenticated**: 認証済みユーザーが必要
- **IsAdminUser**: 認証済み管理者ユーザーが必要
- **IsActiveUser**: 認証済みかつアクティブなユーザーが必要
- **IsAuthenticatedOrReadOnly**: 書き込みには認証が必要、匿名ユーザーは読み取り専用

```rust
use reinhardt_auth::{Permission, IsAuthenticated, PermissionContext};

let permission = IsAuthenticated;
let context = PermissionContext {
    request: &request,
    is_authenticated: true,
    is_admin: false,
    is_active: true,
};

assert!(permission.has_permission(&context).await);
```

### エラーハンドリング

#### AuthenticationError

- **InvalidCredentials**: ユーザー名またはパスワードが間違っている
- **UserNotFound**: ユーザーが存在しない
- **SessionExpired**: セッションが期限切れ
- **InvalidToken**: トークンが不正または無効
- **Unknown**: カスタムメッセージ付きの一般的なエラー

#### AuthenticationBackendトレイト

- **統一されたエラーハンドリング**: すべてのバックエンドが`AuthenticationError`を使用
- **標準エラートレイト**: `std::error::Error`を実装
- **Display実装**: ユーザーフレンドリーなエラーメッセージ

### セッションベース認証

#### SessionAuthentication

- **セッション管理**: HashMapベースのデータストレージを持つ`Session`構造体
- **SessionStoreトレイト**: セッション永続化のための非同期インターフェース
  - `load()`: IDでセッションを取得
  - `save()`: セッションデータを永続化
  - `delete()`: セッションを削除
- **InMemorySessionStore**: 組み込みのインメモリセッションストレージ
- **SessionId**: 型安全なセッション識別子ラッパー
- **Cookie統合**: セキュアなセッションCookieハンドリング

```rust
use reinhardt_auth::{SessionAuthentication, Session, InMemorySessionStore};

let store = InMemorySessionStore::new();
let auth = SessionAuthentication::new(store);

// セッション作成
let session_id = auth.create_session(user).await?;

// セッション検証
let user = auth.authenticate_session(&session_id).await?;
```

### 多要素認証 (MFA)

#### TOTPベースMFA

- **MFAAuthentication**: 時間ベースのワンタイムパスワード（TOTP）認証
- **シークレット管理**: ユーザーごとのセキュアなシークレット保存
- **QRコード生成**: 認証アプリ（Google Authenticator、Authyなど）用のTOTP URLを生成
- **コード検証**: 設定可能な時間ウィンドウでTOTPコードを検証
- **登録フロー**: シークレット生成によるユーザー登録
- **時間ウィンドウ**: 時刻のずれに対する許容範囲を設定可能（デフォルト: 1タイムステップ）

```rust
use reinhardt_auth::MFAAuthentication;

let mfa = MFAAuthentication::new("MyApp");

// MFA用のユーザー登録
let totp_url = mfa.register_user("alice").await?;
// ユーザーはtotp_urlから生成されたQRコードをスキャン

// ログイン時のコード検証
let code = "123456"; // ユーザーの認証アプリから
assert!(mfa.verify_code("alice", code).await?);
```

### OAuth2サポート

#### OAuth2認証

- **OAuth2Authentication**: 完全なOAuth2プロバイダー実装
- **グラントタイプ**: 認可コード、クライアント認証情報、リフレッシュトークン、インプリシット
- **アプリケーション管理**: クライアント認証情報を持つ`OAuth2Application`
- **トークン管理**: アクセストークンとリフレッシュトークンを持つ`OAuth2Token`
- **認可フロー**:
  - 認可コードの生成と検証
  - トークン交換（コード → アクセストークン）
  - リフレッシュトークンによるトークン更新
- **OAuth2TokenStoreトレイト**: 永続的なトークンストレージインターフェース
- **InMemoryTokenStore**: 組み込みのインメモリトークンストレージ

```rust
use reinhardt_auth::{OAuth2Authentication, GrantType, InMemoryTokenStore};

let store = InMemoryTokenStore::new();
let oauth2 = OAuth2Authentication::new(store);

// OAuth2アプリケーションの登録
oauth2.register_application(
    "client123",
    "secret456",
    "https://example.com/callback",
    vec![GrantType::AuthorizationCode]
).await?;

// 認可コードフロー
let code = oauth2.generate_authorization_code("client123", "user123", vec!["read", "write"]).await?;
let token = oauth2.exchange_code(&code, "client123").await?;

// アクセストークンの使用
let claims = oauth2.verify_token(&token.access_token).await?;
```

### トークンブラックリスト & ローテーション

#### トークンブラックリスト

- **TokenBlacklistトレイト**: トークン無効化のためのインターフェース
- **BlacklistReason**: カテゴライズされた失効理由
  - `Logout`: ユーザーによるログアウト
  - `Compromised`: セキュリティインシデント
  - `ManualRevoke`: 管理者による失効
  - `Rotated`: 自動トークンローテーション
- **InMemoryBlacklist**: 組み込みのインメモリブラックリストストレージ
- **クリーンアップ**: 期限切れブラックリストエントリの自動削除
- **統計**: 使用状況の追跡と監視

#### トークンローテーション

- **TokenRotationManager**: 自動リフレッシュトークンローテーション
- **RefreshTokenStoreトレイト**: 永続的なリフレッシュトークンストレージ
- **ローテーションフロー**: 新しいトークン発行時に古いトークンを無効化
- **セキュリティ**: リフレッシュトークンの再利用攻撃を防止
- **InMemoryRefreshStore**: 組み込みのインメモリリフレッシュトークンストレージ

```rust
use reinhardt_auth::{
    TokenBlacklist, InMemoryBlacklist, BlacklistReason,
    TokenRotationManager, InMemoryRefreshStore
};

// トークンブラックリスト
let blacklist = InMemoryBlacklist::new();
blacklist.blacklist("old_token", BlacklistReason::Logout).await?;
assert!(blacklist.is_blacklisted("old_token").await?);

// トークンローテーション
let refresh_store = InMemoryRefreshStore::new();
let rotation_manager = TokenRotationManager::new(blacklist, refresh_store);

let new_token = rotation_manager.rotate_token("old_refresh_token", "user123").await?;
```

### リモートユーザー認証

#### ヘッダーベース認証

- **RemoteUserAuthentication**: 信頼できるHTTPヘッダー経由での認証
- **リバースプロキシ統合**: 認証プロキシ（nginx、Apacheなど）のサポート
- **ヘッダー設定**: 設定可能なヘッダー名（デフォルト: `REMOTE_USER`）
- **ヘッダー検証**: ヘッダーの存在と形式を検証
- **自動ログアウト**: ヘッダーがない場合の強制ログアウト（オプション）
- **SSOサポート**: シングルサインオン統合

```rust
use reinhardt_auth::RemoteUserAuthentication;

// 標準設定
let auth = RemoteUserAuthentication::new("REMOTE_USER");

// 強制ログアウト付き
let auth = RemoteUserAuthentication::new("REMOTE_USER").force_logout_if_no_header(true);

// リクエストから認証
let user = auth.authenticate(&request).await?;
```

## 予定

以下の機能は将来のリリースで実装予定です：

### トークンベース認証

- **TokenAuthentication**: APIトークン認証
- **Token Storage**: トークンの永続化とルックアップ
- **Token Rotation**: セキュリティのための自動トークンローテーション

### 高度なパーミッション

- **RateLimitPermission**: IPまたはユーザーによるリクエストレート制限
- **TimeBasedPermission**: 時間帯によるアクセス制御
- **IpWhitelistPermission**: IPベースのアクセス制御
- **IpBlacklistPermission**: IPブロッキング
- **Permission Operators**: 複雑なロジックのためのAND、OR、NOT演算子

### モデルパーミッション

- **DjangoModelPermissions**: Djangoスタイルのモデルパーミッション
- **DjangoModelPermissionsOrAnonReadOnly**: 匿名読み取りアクセス
- **ModelPermission**: モデルごとのCRUDパーミッション
- **Permission Checking**: オブジェクトレベルのパーミッションサポート

### Django REST Framework互換性

- **DRF Authentication Classes**: 互換性のある認証インターフェース
- **DRF Permission Classes**: 互換性のあるパーミッションインターフェース
- **Browsable API Support**: DRFスタイルのブラウザブルAPI統合

### 管理とマネジメント

- **User Management**: ユーザーのCRUD操作
- **Group Management**: ユーザーグループとパーミッション
- **Permission Assignment**: ユーザー/グループへのパーミッション割り当て
- **createsuperuser Command**: 管理者ユーザー作成のためのCLIツール

## 使用例

### 完全な認証フロー

```rust
use reinhardt_auth::{
    JwtAuth, HttpBasicAuth, AuthBackend,
    SimpleUser, User, Argon2Hasher, PasswordHasher,
    Permission, IsAuthenticated, PermissionContext
};

// 1. JWT認証のセットアップ
let jwt_auth = JwtAuth::new(b"secret-key");

// 2. ユーザー付きBasic認証のセットアップ
let mut basic_auth = HttpBasicAuth::new();
basic_auth.add_user("alice", "password123");

// 3. ユーザー認証とJWT生成
let user = basic_auth.authenticate(&request).unwrap().unwrap();
let token = jwt_auth.generate_token(
    user.id(),
    user.username().to_string()
).unwrap();

// 4. 後続のリクエストでトークンを検証
let claims = jwt_auth.verify_token(&token).unwrap();

// 5. パーミッションチェック
let permission = IsAuthenticated;
let context = PermissionContext {
    request: &request,
    is_authenticated: true,
    is_admin: user.is_admin(),
    is_active: user.is_active(),
};

if permission.has_permission(&context).await {
    // アクセス許可
}
```

### カスタム認証バックエンド

```rust
use reinhardt_auth::{AuthBackend, SimpleUser, Argon2Hasher, PasswordHasher};
use async_trait::async_trait;
use std::collections::HashMap;

struct MyAuthBackend {
    users: HashMap<String, (String, SimpleUser)>,
    hasher: Argon2Hasher,
}

#[async_trait]
impl AuthBackend for MyAuthBackend {
    type User = SimpleUser;

    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> reinhardt_apps::Result<Option<Self::User>> {
        if let Some((hash, user)) = self.users.get(username) {
            if self.hasher.verify(password, hash)? {
                return Ok(Some(user.clone()));
            }
        }
        Ok(None)
    }

    async fn get_user(&self, user_id: &str)
        -> reinhardt_apps::Result<Option<Self::User>> {
        Ok(self.users.values()
            .find(|(_, u)| u.id.to_string() == user_id)
            .map(|(_, u)| u.clone()))
    }
}
```

## ライセンス

以下のいずれかのライセンスの下でライセンスされています：

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好きな方を選択してください。
