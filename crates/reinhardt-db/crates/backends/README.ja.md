# backends

Reinhardt ORM用のデータベースバックエンド実装

## 概要

`backends`は、Reinhardt ORMレイヤー用のデータベースバックエンド実装を提供します。PostgreSQL、MySQL、SQLiteデータベースのサポートと、クエリ構築および実行のための統一された抽象化を含みます。

## 機能

- PostgreSQLバックエンド実装
- MySQLバックエンド実装
- SQLiteバックエンド実装
- 統一されたデータベース抽象化レイヤー
- sea-queryとのクエリビルダー統合
- sqlxによる型安全なパラメータバインディング

## インストール

```toml
[dependencies]
backends = "0.1.0"
```

### 機能

- `postgres` (デフォルト): PostgreSQLサポート
- `mysql`: MySQLサポート
- `sqlite`: SQLiteサポート
- `all-databases`: すべてのデータベースバックエンド

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかの条件の下でライセンスされています。
