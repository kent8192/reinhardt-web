# reinhardt-database

データベース抽象化レイヤー

## 概要

SQL及びNoSQLデータベースの統一インターフェースを提供します。PostgreSQL、MySQL、SQLite、MongoDB、Redisをサポートし、トレイト継承による拡張可能な設計を採用しています。

## 機能

- PostgreSQL、MySQL、SQLiteのサポート
- MongoDB、Redisのサポート（オプション）
- トレイト継承による拡張可能な設計
- データベース固有の機能へのフルアクセス
- 型安全なデータベース操作
- 統一されたインターフェース