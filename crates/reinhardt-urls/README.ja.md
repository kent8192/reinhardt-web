# reinhardt-urls

ReinhardtフレームワークのURLルーティングおよびプロキシユーティリティ

## 概要

`reinhardt-urls`は、Reinhardtアプリケーション向けの包括的なURLルーティングおよびリレーションシッププロキシ機能を提供します。Django風のURLルーティングとSQLAlchemy風のアソシエーションプロキシを組み合わせ、リレーションシップを通じた透過的な属性アクセスを実現します。

## 機能

このクレートは以下のサブクレートから機能を再エクスポートしています：

- **Routers** (`reinhardt-routers`): 自動URLルーティング設定
  - 組み合わせ可能なルーターインターフェースのための`Router` trait
  - 自動ViewSet URL生成付きの`DefaultRouter`
  - 自動リスト/詳細エンドポイント生成（`/resource/`と`/resource/{id}/`）
  - カスタムViewSetアクションサポート（リストと詳細レベル）
  - パスパラメータ抽出付きリクエストディスパッチ
  - オプショナルな名前空間付き名前付きルート
  - URL逆引き機能
  - 名前空間パターンからのバージョン抽出

- **Routers Macros** (`reinhardt-routers-macros`): ルーティング用の手続き型マクロ
  - ルート定義のための`#[route]`マクロ
  - コンパイル時ルート検証
  - 型安全なルート生成

- **Proxy** (`reinhardt-proxy`): リレーションシップトラバーサル用のアソシエーションプロキシ
  - 透過的な属性アクセスのための`AssociationProxy<T, U>`
  - 一対一および多対一リレーションシップのための`ScalarProxy`
  - 一対多および多対多リレーションシップのための`CollectionProxy`
  - 豊富な比較演算子（Eq、Ne、Gt、Gte、Lt、Lte、In、NotIn等）
  - 非同期get/set操作
  - コレクション操作メソッド

## サブクレート

このクレートは以下のサブクレートを含んでいます：

```
reinhardt-urls/
├── Cargo.toml          # 親クレート定義
├── src/
│   └── lib.rs          # サブクレートからの再エクスポート
└── crates/
    ├── routers/        # URLルーティング
    ├── routers-macros/ # ルーティングマクロ
    └── proxy/          # アソシエーションプロキシ
```
