# Reinhardt Configuration Framework

シークレット管理、暗号化、監査ログを備えたDjango風の設定管理システム

## 概要

`reinhardt-conf`クレートは、Djangoの設定システムに着想を得た、追加のセキュリティ機能を備えたReinhardtアプリケーション向けの包括的な設定管理フレームワークを提供します。

## 機能

- 複数の設定ソース（ファイル、環境変数、コマンドライン引数）
- 型安全な設定（カスタムバリデータによる強力な型検証）
- シークレット管理（HashiCorp Vault、AWS Secrets Manager、Azure Key Vault統合）
- 暗号化（機密設定用の組み込み暗号化）
- 動的バックエンド（Redis及びデータベースバックドの動的設定）
- シークレットローテーション（自動シークレットローテーションサポート）
- 監査ログ（すべての設定変更を追跡）

## サブクレート

このクレートは、以下のサブクレートを含む親クレートとして構成されています：

- **`settings`** (`reinhardt-settings`): コア設定管理機能
- **`settings-cli`** (`reinhardt-settings-cli`): 設定管理用CLIツール