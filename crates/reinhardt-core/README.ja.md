# reinhardt-core

Reinhardtフレームワークのコアコンポーネント

## 概要

`reinhardt-core`は、Reinhardtフレームワークの基本的なビルディングブロックを提供します。他のクレートが依存する、必須の型、トレイト、エラーハンドリング、シグナル、セキュリティプリミティブ、バリデータ、バックエンド抽象化が含まれます。

## 機能

このクレートは以下のサブクレートから機能を再エクスポートしています：

- **Types** (`reinhardt-types`): コア型定義
  - ハンドラートレイト
  - ミドルウェアトレイト
  - ミドルウェアチェイン
- **Exception** (`reinhardt-exception`): 例外処理とエラー型
  - Django風の例外階層
  - HTTP例外（401、403、404、500等）
  - バリデーションエラー
  - データベース例外
- **Signals** (`reinhardt-signals`): イベント駆動フック
  - 型安全なシグナルシステム
  - モデル、マイグレーション、リクエストのライフサイクルシグナル
  - 非同期及び同期シグナルディスパッチ
- **Macros** (`reinhardt-macros`): 手続き型マクロ
  - `#[handler]` マクロ
  - `#[middleware]` マクロ
  - `#[injectable]` マクロ
- **Security** (`reinhardt-security`): セキュリティプリミティブ
  - パスワードハッシュと検証
  - CSRF保護
  - XSS防止
- **Validators** (`reinhardt-validators`): データバリデーション
  - メール検証
  - URL検証
  - 長さバリデータ
  - 範囲バリデータ
- **Backends** (`reinhardt-backends`): バックエンド抽象化
  - キャッシュバックエンドトレイト
  - セッションバックエンドトレイト
  - Redisバックエンド実装