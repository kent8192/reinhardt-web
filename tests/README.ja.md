# reinhardt-integration-tests

Reinhardtフレームワークの統合テスト

## 概要

複数のReinhardtクレート間の相互作用をテストするための包括的な統合テストスイートです。TestContainersを使用して実際のインフラストラクチャで現実的なシナリオをテストします。

## 機能

- 複数クレート間の統合テスト
- TestContainersによる実インフラストラクチャテスト
- データベース統合テスト（PostgreSQL、MySQL、SQLite）
- HTTPサーバー統合テスト
- 認証と認可のフローテスト
- シリアライザーとORMの統合テスト
- テンプレートレンダリング統合テスト
- エンドツーエンドAPIワークフローテスト
