# reinhardt-admin

Reinhardtプロジェクト管理用のグローバルコマンドラインツール。

## 概要

`reinhardt-admin`は、ReinhardtにおけるDjangoの`django-admin`相当のツールです。新しいプロジェクトやアプリケーションを作成するユーティリティを提供します。

## インストール

cargoを使用してグローバルにインストール:

```bash
cargo install reinhardt-admin
```

## 使用方法

### 新しいプロジェクトを作成

```bash
# RESTful APIプロジェクトを作成 (デフォルト)
reinhardt-admin startproject myproject

# MTVスタイルのプロジェクトを作成
reinhardt-admin startproject myproject --template-type mtv

# 特定のディレクトリにプロジェクトを作成
reinhardt-admin startproject myproject /path/to/directory
```

### 新しいアプリを作成

```bash
# RESTfulアプリを作成 (デフォルト)
reinhardt-admin startapp myapp

# MTVスタイルのアプリを作成
reinhardt-admin startapp myapp --template-type mtv

# 特定のディレクトリにアプリを作成
reinhardt-admin startapp myapp /path/to/directory
```

### その他のコマンド

```bash
# ヘルプを表示
reinhardt-admin help

# バージョンを表示
reinhardt-admin --version
```

## Djangoとの対応

| Django | Reinhardt |
|--------|-----------|
| `django-admin startproject myproject` | `reinhardt-admin startproject myproject` |
| `django-admin startapp myapp` | `reinhardt-admin startapp myapp` |

## プロジェクトテンプレート

`reinhardt-admin`には2つのプロジェクトテンプレートが含まれています:

- **RESTful** (デフォルト): API中心のアプリケーション
- **MTV**: 伝統的なサーバーレンダリングWebアプリケーション (Model-Template-View)

## アプリテンプレート

アプリは2つの形式で作成できます:

- **モジュール** (デフォルト): `apps/`ディレクトリに作成
- **ワークスペース**: ワークスペース内の独立したクレート

## 機能

- **埋め込みテンプレート**: `rust-embed`を使用してテンプレートをバイナリにコンパイル
- **外部依存なし**: インターネット接続なしで動作
- **Django互換**: Django開発者にとって馴染みのあるインターフェース

## アーキテクチャ

`reinhardt-admin`はコア機能を`reinhardt-commands`に依存しています:

```
reinhardt-admin (CLIバイナリ)
    ↓
reinhardt-commands (ライブラリ)
    ↓
StartProjectCommand / StartAppCommand
```

## ライセンス

以下のいずれかのライセンスで提供:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好きな方を選択してください。
