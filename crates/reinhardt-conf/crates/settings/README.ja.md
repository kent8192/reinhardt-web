# reinhardt-settings

Django風の設定管理システムで、シークレット管理、暗号化、監査ログ、動的設定などの高度な機能を備えたRust実装です。

[![Crates.io](https://img.shields.io/crates/v/reinhardt-settings.svg)](https://crates.io/crates/reinhardt-settings)
[![Documentation](https://docs.rs/reinhardt-settings/badge.svg)](https://docs.rs/reinhardt-settings)
[![License](https://img.shields.io/crates/l/reinhardt-settings.svg)](LICENSE)

## 機能の状態

### コア機能（実装済み ✓）

- **📁 階層的な設定**: TOML、JSON、.envファイルのサポートと環境固有のオーバーライド
  - ✓ TOMLファイルソース（`TomlFileSource`）
  - ✓ JSONファイルソース（`JsonFileSource`）
  - ✓ 変数展開サポート付き.envファイルローダー（`EnvLoader`、`DotEnvSource`）
  - ✓ 環境変数ソース（`EnvSource`）
  - ✓ デフォルト値ソース（`DefaultSource`）
  - ✓ ファイル拡張子による設定フォーマットの自動検出
  - ✓ 優先度ベースの設定マージ

- **🌍 環境プロファイル**: 開発、ステージング、本番環境の組み込みプロファイル
  - ✓ プロファイル列挙型（Development、Staging、Production、Custom）
  - ✓ `REINHARDT_ENV`、`ENVIRONMENT`、`REINHARDT_SETTINGS_MODULE`からの環境検出
  - ✓ プロファイル固有の.envファイルローディング（`.env.development`、`.env.production`など）
  - ✓ プロファイル対応のデフォルト設定

- **✅ バリデーション**: プロファイル固有のセキュリティ検証
  - ✓ 必須フィールドの検証（`RequiredValidator`）
  - ✓ 本番環境向けセキュリティ検証（`SecurityValidator`）
  - ✓ 数値の範囲検証（`RangeValidator`）
  - ✓ 正規表現によるパターン検証（`PatternValidator`）
  - ✓ 列挙型のような値の選択肢検証（`ChoiceValidator`）
  - ✓ `reinhardt-validators`クレートとの統合

- **🎯 型安全**: serdeとの統合による完全なRust型安全性
  - ✓ Django互換の`Settings`構造体
  - ✓ データベース設定（SQLite、PostgreSQL、MySQL）
  - ✓ テンプレートエンジン設定
  - ✓ ミドルウェア設定
  - ✓ Serdeシリアライズ/デシリアライズのサポート

### 高度な機能（実装済み ✓）

- **🔐 シークレット管理**: HashiCorp Vault、AWS Secrets Manager、Azure Key Vaultの統合サポート
  - ✓ 自動的にマスキングされるシークレット型（`SecretString`、`SecretValue`）
  - ✓ タイミング攻撃防止のための定数時間等価性比較
  - ✓ メモリセキュリティのためのドロップ時ゼロ化
  - ✓ シークレットプロバイダートレイト（`SecretProvider`）
  - ✓ 環境変数プロバイダー（`env::EnvSecretProvider`）
  - ✓ テスト用メモリプロバイダー（`memory::MemorySecretProvider`）
  - ✓ HashiCorp Vaultプロバイダー（機能: `vault`）
  - ✓ AWS Secrets Managerプロバイダー（機能: `aws-secrets`）
  - ✓ Azure Key Vaultプロバイダー（機能: `azure-keyvault`）
  - ✓ シークレットローテーションサポート（機能: `secret-rotation`）
  - ✓ シークレットアクセスの監査ログ

- **🔒 暗号化**: 機密設定用のAES-256-GCMファイル暗号化
  - ✓ 設定暗号化器（`ConfigEncryptor`）
  - ✓ 暗号化された設定構造体（`EncryptedConfig`）
  - ✓ 鍵ベースの暗号化/復号化（機能: `encryption`）

- **📝 監査ログ**: コンプライアンスのためのすべての設定変更の追跡
  - ✓ 監査イベントタイプ（Read、Write、Deleteなど）
  - ✓ 監査バックエンドトレイト（`AuditBackend`）
  - ✓ ファイルベース監査バックエンド（`FileAuditBackend`）
  - ✓ データベース監査バックエンド（`DatabaseAuditBackend`）
  - ✓ テスト用メモリ監査バックエンド（`MemoryAuditBackend`）
  - ✓ シークレット用の個別監査ログ

### 動的機能

- **⚡ 動的設定**: 複数のストレージバックエンドによる実行時設定変更
  - ✓ バックエンドトレイト定義（`DynamicBackend`）
  - ✓ TTLサポート付きメモリバックエンド（`MemoryBackend`）
  - ✓ Redisバックエンド（機能: `dynamic-redis`）
  - ✓ データベースバックエンド（機能: `dynamic-database`）
  - ✓ 型安全なジェネリクスを使ったCRUD操作
  - ✓ 変更通知のためのオブザーバーパターン
  - ✓ TTL付きLRUキャッシング（機能: `caching`）

- **🔄 ホットリロード**: 自動設定リロードのためのファイルシステム監視
  - ✓ `notify`クレートによるファイル監視（機能: `hot-reload`）
  - ✓ 急速なファイル変更のデバウンス処理
  - ✓ コールバックベースの通知システム
  - ✓ DynamicSettingsとの統合

- **🛠️ CLIツール**: 設定管理のためのコマンドラインユーティリティ（`reinhardt-settings-cli`で実装）
  - ✓ 設定検証CLI（`validate`コマンド）
  - ✓ 設定表示CLI（`show`コマンド）
  - ✓ 設定変更CLI（`set`コマンド）
  - ✓ 設定比較CLI（`diff`コマンド）
  - ✓ シークレット管理CLI（`encrypt`、`decrypt`コマンド）
  - 詳細は[reinhardt-settings-cli](../settings-cli/README.md)を参照してください

## インストール

`Cargo.toml`に追加:

```toml
[dependencies]
reinhardt-settings = "0.1.0"

# すべての機能を有効にする
reinhardt-settings = { version = "0.1.0", features = ["full"] }

# 特定の機能を有効にする
reinhardt-settings = { version = "0.1.0", features = ["async", "encryption", "vault"] }
```

## 機能フラグ

### コア機能

- `async` - 非同期サポート（動的バックエンドとシークレット管理に必要）

### 動的設定バックエンド

- `dynamic-redis` - 実行時設定変更用のRedisバックエンド（`async`が必要）
- `dynamic-database` - 動的設定用のsqlxを使用したSQLバックエンド（`async`が必要）

### シークレット管理プロバイダー

- `vault` - シークレットストレージのためのHashiCorp Vault統合（`async`が必要）
- `aws-secrets` - AWS Secrets Manager統合（`async`が必要）
- `azure-keyvault` - Azure Key Vault統合（`async`が必要）
- `secret-rotation` - 自動シークレットローテーション機能（`async`が必要）

### セキュリティ機能

- `encryption` - PBKDF2鍵導出によるAES-256-GCMファイル暗号化

### 組み合わせ例

```toml
# すべてのシークレットプロバイダーを含む完全な非同期機能
reinhardt-settings = { version = "0.1.0", features = ["async", "vault", "aws-secrets", "azure-keyvault", "encryption"] }

# Redisを使った動的設定
reinhardt-settings = { version = "0.1.0", features = ["dynamic-redis", "encryption"] }

# 暗号化のみの最小構成
reinhardt-settings = { version = "0.1.0", features = ["encryption"] }
```

## クイックスタート

### 基本的な設定

```rust
use reinhardt_settings::Settings;
use std::path::PathBuf;

fn main() {
    // 基本的な設定の作成
    let settings = Settings::new(
        PathBuf::from("/app"),
        "your-secret-key-here".to_string()
    )
    .with_root_urlconf("myapp.urls");

    println!("Debug mode: {}", settings.debug);
    println!("Database: {}", settings.databases.get("default").unwrap().name);
}
```

### 設定ソースの使用

```rust
use reinhardt_settings::sources::{TomlFileSource, EnvSource, ConfigSource};
use reinhardt_settings::profile::Profile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TOMLファイルから読み込み
    let toml_source = TomlFileSource::new("settings.toml");
    let toml_config = toml_source.load()?;

    // プレフィックス付き環境変数から読み込み
    let env_source = EnvSource::new().with_prefix("APP_");
    let env_config = env_source.load()?;

    // 設定ソースは優先度によってマージされます
    // EnvSource (優先度 100) > TomlFileSource (優先度 50)

    Ok(())
}
```

### 環境プロファイル

```rust
use reinhardt_settings::profile::Profile;
use reinhardt_settings::sources::DotEnvSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 環境からプロファイルを検出
    let profile = Profile::from_env().unwrap_or(Profile::Development);

    // プロファイル固有の.envファイルを読み込み
    let env_source = DotEnvSource::new()
        .with_profile(profile)
        .with_interpolation(true);

    env_source.load()?;

    println!("Running in {} mode", profile);
    println!("Debug enabled: {}", profile.default_debug());

    Ok(())
}
```

### バリデーション

```rust
use reinhardt_settings::validation::{SecurityValidator, SettingsValidator};
use reinhardt_settings::profile::Profile;
use std::collections::HashMap;
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = HashMap::new();
    settings.insert("debug".to_string(), Value::Bool(false));
    settings.insert("secret_key".to_string(), Value::String("a-very-long-and-secure-secret-key-here".to_string()));
    settings.insert("allowed_hosts".to_string(), Value::Array(vec![
        Value::String("example.com".to_string())
    ]));

    // 本番環境向けの検証
    let validator = SecurityValidator::new(Profile::Production);
    validator.validate_settings(&settings)?;

    println!("Settings validated successfully!");

    Ok(())
}
```

## 高度な使用法

### シークレット管理

`async`機能を有効にすると、シークレットプロバイダーを使用できます:

```rust
use reinhardt_settings::secrets::{SecretString, SecretProvider};
use reinhardt_settings::secrets::providers::memory::MemorySecretProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = MemorySecretProvider::new();

    // シークレットの保存
    let secret = SecretString::new("my-database-password");
    provider.set_secret("db_password", secret).await?;

    // シークレットの取得
    let retrieved = provider.get_secret("db_password").await?;

    // シークレットはログで自動的にマスキングされます
    println!("Secret: {}", retrieved); // 出力: [REDACTED]

    // 必要な場合は実際の値にアクセス
    println!("Actual: {}", retrieved.expose_secret());

    Ok(())
}
```

### 設定の暗号化

`encryption`機能を使用:

```rust
use reinhardt_settings::encryption::ConfigEncryptor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = vec![0u8; 32]; // 本番環境では安全な鍵を使用してください
    let encryptor = ConfigEncryptor::new(key)?;

    // 設定データの暗号化
    let data = b"secret configuration";
    let encrypted = encryptor.encrypt(data)?;

    // 必要な時に復号化
    let decrypted = encryptor.decrypt(&encrypted)?;

    assert_eq!(data, decrypted.as_slice());

    Ok(())
}
```

### 監査ログ

コンプライアンスのための設定変更の追跡:

```rust
use reinhardt_settings::audit::backends::memory::MemoryAuditBackend;
use reinhardt_settings::audit::{AuditEvent, AuditBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = MemoryAuditBackend::new();

    // 設定変更のログ記録
    let event = AuditEvent::write("database.host", "localhost", "db.example.com");
    backend.log(event).await?;

    // 監査ログのクエリ
    let logs = backend.query_all().await?;

    for log in logs {
        println!("Event: {:?}", log);
    }

    Ok(())
}
```

## ドキュメント

完全なドキュメントとAPIリファレンスは以下を参照してください:

- [APIドキュメント](https://docs.rs/reinhardt-settings)

## モジュール構造

このクレートは以下のモジュールで構成されています:

- **コアモジュール**
  - `config` - 設定トレイト定義
  - `env` - 環境変数ユーティリティ
  - `env_loader` - 変数展開付き.envファイルローディング
  - `env_parser` - 環境変数パース
  - `profile` - 環境プロファイル管理（Development、Staging、Production）
  - `sources` - 設定ソース実装（TOML、JSON、.env、環境変数）
  - `validation` - 設定検証フレームワーク
  - `builder` - 設定構築のための流暢なビルダー
  - `prelude` - 便利な共通インポート
  - `testing` - テストユーティリティ

- **高度なモジュール**（機能フラグで制御）
  - `advanced` - 高度な設定構造体（キャッシュ、CORS、メール、ログ、メディア、セッション、静的ファイル）
  - `encryption` - AES-256-GCM設定暗号化（機能: `encryption`）
  - `audit` - 設定変更の監査ログ（機能: `async`）
    - `backends` - ファイル、データベース、メモリ監査バックエンド
  - `secrets` - シークレット管理システム（機能: `async`）
    - `types` - 自動マスキング付きSecretString、SecretValue
    - `providers` - HashiCorp Vault、AWS Secrets Manager、Azure Key Vault、環境変数、メモリ
    - `rotation` - 自動シークレットローテーション（機能: `secret-rotation`）
    - `audit` - シークレットアクセスの監査ログ
  - `backends` - 動的設定バックエンド（機能: `async`）
    - `memory` - テスト用インメモリバックエンド
    - `redis_backend` - Redisバックエンド（機能: `dynamic-redis`）
    - `database` - SQLバックエンド（機能: `dynamic-database`）
  - `dynamic` - 実行時設定変更（機能: `async`）

## テスト

テストの実行:

```bash
# ユニットテスト
cargo test --package reinhardt-settings

# 特定の機能付き
cargo test --package reinhardt-settings --features encryption
cargo test --package reinhardt-settings --features async

# すべての機能付き
cargo test --package reinhardt-settings --all-features

# 統合テスト（Redis/DatabaseバックエンドにはDockerが必要）
cargo test --package reinhardt-settings --test integration_test --features encryption
cargo test --package reinhardt-settings --test integration_test --all-features
```

### テストカバレッジ

- **コア機能**: 40以上のユニットテスト
- **シークレット管理**: 定数時間等価性とゼロ化を含む20以上のテスト
- **バリデーション**: セキュリティ、範囲、パターン、選択肢バリデーター用の10以上のテスト
- **設定ソース**: TOML、JSON、.env、環境変数ソース用の15以上のテスト
- **プロファイル管理**: 環境検出とプロファイル動作用の10以上のテスト
- **統合テスト**: 暗号化、Redisバックエンド、Databaseバックエンド

## アーキテクチャのハイライト

### セキュリティファーストの設計

- **シークレット保護**: `SecretString`と`SecretValue`型がログでの偶発的な露出を防止
- **定数時間比較**: シークレット等価性チェックのタイミング攻撃防止
- **メモリゼロ化**: `zeroize`クレートを使用した機密データの自動クリーンアップ
- **本番環境検証**: 本番環境向けの自動セキュリティチェック

### 柔軟な設定

- **優先度ベースのマージ**: 環境変数（100）> .envファイル（90）> 設定ファイル（50）> デフォルト（0）
- **複数のソース**: TOML、JSON、.envファイル、環境変数
- **プロファイル対応**: 開発、ステージング、本番環境の異なるデフォルト値
- **型安全**: serdeとの統合による完全なRust型安全性

### 拡張可能なバックエンドシステム

- **プラグイン可能なプロバイダー**: 新しいシークレットプロバイダーや設定バックエンドの追加が容易
- **非同期対応**: I/O操作の完全なasync/awaitサポート
- **監査証跡**: 設定とシークレットアクセスの完全なログ記録
- **テストサポート**: 簡単なテストのためのメモリベースバックエンド

## パフォーマンスの考慮事項

- **ゼロコスト抽象化**: 型安全性のための実行時オーバーヘッドなし
- **遅延ロード**: 設定ソースは必要な時のみロード
- **効率的なマージ**: IndexMapベースのマージで挿入順序を維持
- **最小限のアロケーション**: 文字列のアロケーションとクローンの慎重な使用

## コントリビューション

コントリビューションを歓迎します！以下の分野でヘルプが必要です:

- 設定管理のためのCLIツール
- 追加のシークレットプロバイダー実装
- 動的設定のためのホットリロード実装
- パフォーマンスの最適化
- ドキュメントの改善

## ライセンス

以下のいずれかでライセンスされています:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好みの方を選択してください。

## 謝辞

以下からインスピレーションを得ています:

- [Django Settings](https://docs.djangoproject.com/en/stable/ref/settings/)
- [django-environ](https://django-environ.readthedocs.io/)
- [config-rs](https://github.com/mehcode/config-rs)