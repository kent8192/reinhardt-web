# Reinhardt Configuration Framework

シークレット管理、暗号化、監査ログを備えたDjango風の設定管理システム

## 概要

`reinhardt-conf`クレートは、Djangoの設定システムに着想を得た、追加のセキュリティ機能を備えたReinhardtアプリケーション向けの包括的な設定管理フレームワークを提供します。

## 機能

- **複数の設定ソース**: ファイル、環境変数、コマンドライン引数
- **型安全な設定**: カスタムバリデータによる強力な型検証
- **シークレット管理**: HashiCorp Vault、AWS Secrets Manager、Azure Key Vaultとの統合
- **暗号化**: 機密設定用の組み込み暗号化
- **動的バックエンド**: Redisおよびデータベースバックドの動的設定
- **シークレットローテーション**: 自動シークレットローテーションサポート
- **監査ログ**: すべての設定変更を追跡

## サブクレート

このクレートは、以下のサブクレートを含む親クレートとして構成されています：

- **`settings`** (`reinhardt-settings`): コア設定管理機能
- **`settings-cli`** (`reinhardt-settings-cli`): 設定管理用CLIツール

## インストール

`Cargo.toml`に以下を追加してください：

```toml
[dependencies]
reinhardt-conf = "0.1.0"
```

### オプション機能

必要に応じて特定の機能を有効化できます：

```toml
[dependencies]
reinhardt-conf = { version = "0.1.0", features = ["async", "encryption"] }
```

利用可能な機能：

- `settings` (デフォルト): コア設定機能
- `async`: 非同期設定操作
- `dynamic-redis`: Redisバックドの動的設定
- `dynamic-database`: データベースバックドの動的設定
- `vault`: HashiCorp Vault統合
- `aws-secrets`: AWS Secrets Manager統合
- `azure-keyvault`: Azure Key Vault統合
- `secret-rotation`: 自動シークレットローテーション
- `encryption`: 機密設定用の組み込み暗号化

## 使用方法

```rust
use reinhardt_conf::SettingsBuilder;

// 基本的な使用方法
let settings = SettingsBuilder::new()
    .add_source(ConfigSource::File("config.toml"))
    .add_source(ConfigSource::Environment)
    .build()?;

// 設定へのアクセス
let database_url = settings.get::<String>("DATABASE_URL")?;
```

## CLIツール

`settings-cli`サブクレートは、設定管理用のコマンドラインツールを提供します：

```bash
# CLIツールのインストール
cargo install --path crates/settings-cli

# ツールの使用
reinhardt-settings --help
```

## アーキテクチャ

この親クレートは、サブクレートの機能を再エクスポートします：

```
reinhardt-conf/
├── Cargo.toml          # 親クレート定義
├── src/
│   └── lib.rs          # サブクレートからの再エクスポート
└── crates/
    ├── settings/       # コア設定機能
    └── settings-cli/   # CLIツール
```

## ライセンス

以下のいずれかのライセンスで使用できます：

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

## コントリビューション

コントリビューションを歓迎します！ガイドラインについては、メインの[CONTRIBUTING.md](../../CONTRIBUTING.md)を参照してください。
