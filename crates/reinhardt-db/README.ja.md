# reinhardt-db

Reinhardtフレームワークの Django風データベースレイヤー

## 概要

`reinhardt-db`は、Djangoの ORMに着想を得た、データベース抽象化、オブジェクトリレーショナルマッピング、マイグレーション、コネクションプーリングの強力な機能を備えた、Reinhardtアプリケーション向けの包括的なデータベースレイヤーを提供します。

## 機能

このクレートは以下のサブクレートから機能を再エクスポートしています：

- **Database** (`reinhardt-database`): 低レベルデータベース抽象化レイヤー
  - SQLデータベース用の統一DatabaseBackendトレイト
  - 非同期データベース操作（execute、fetch_one、fetch_all）
  - クエリビルダー（SelectBuilder、InsertBuilder、UpdateBuilder、DeleteBuilder）
  - 型安全なパラメータバインディング
  - コネクションプーリングサポート

- **ORM** (`reinhardt-orm`): オブジェクトリレーショナルマッピングシステム
  - Django風のModelトレイト
  - チェイン可能なクエリのためのQuerySet API
  - フィールド型（AutoField、CharField、IntegerField、DateTimeField等）
  - TimestampedとSoftDeletableトレイト
  - リレーションシップ管理
  - バリデータとchoices

- **Migrations** (`reinhardt-migrations`): スキーママイグレーションシステム
  - モデル変更からの自動マイグレーション生成
  - 前方及び後方マイグレーション
  - スキーマバージョニングと依存関係管理
  - マイグレーション操作（CreateModel、AddField、AlterField等）
  - 状態管理と自動検出

- **Pool** (`reinhardt-pool`): コネクションプール管理
  - データベースコネクションプーリング
  - コネクションライフサイクル管理
  - プール設定とサイジング

- **Hybrid** (`reinhardt-hybrid`): ハイブリッドデータベースサポート
  - マルチデータベースルーティング
  - 読み書き分離
  - データベースシャーディングサポート

- **Associations** (`reinhardt-associations`): リレーションシップ管理
  - 外部キーリレーションシップ
  - 多対多リレーションシップ
  - 一対一リレーションシップ
  - 遅延読み込みと即時読み込み

## サポートされるデータベース

- PostgreSQL
- MySQL
- SQLite