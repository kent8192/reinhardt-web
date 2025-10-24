# backends

Reinhardt ORM用のデータベースバックエンド実装

## 概要

`backends`は、Reinhardt ORMレイヤー用のデータベースバックエンド実装を提供します。クエリ構築と実行のための統一された抽象化により、PostgreSQL、MySQL、SQLiteデータベースのサポートが含まれます。

## 機能

- PostgreSQLバックエンド実装
- MySQLバックエンド実装
- SQLiteバックエンド実装
- 統一されたデータベース抽象化レイヤー
- sea-queryとのクエリビルダー統合
- sqlxによる型安全なパラメータバインディング