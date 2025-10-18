# reinhardt-settings

設定管理システム

## 概要

Django風の設定管理システムで、秘密管理、暗号化、監査ログ、動的設定などの高度な機能を提供します。TOML、JSON、.envファイルをサポートし、環境固有のオーバーライドが可能です。

## 機能

- 階層的な設定管理（TOML、JSON、.env）
- 環境プロファイル（開発、ステージング、本番）
- 動的設定（Redis、SQLバックエンド）
- 秘密管理（Vault、AWS Secrets Manager、Azure Key Vault）
- AES-256-GCM暗号化
- 監査ログ
- CLI管理ツール
- 型安全な設定
