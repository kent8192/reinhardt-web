# reinhardt-messages

Reinhardtフレームワークのためのフラッシュメッセージとユーザー通知

## 概要

Djangoにインスパイアされた、ユーザーに一度だけ表示される通知メッセージを扱うメッセージングフレームワークです。複数のストレージバックエンドと柔軟な設定オプションを備えた完全なメッセージシステムを提供します。

## 機能

### 実装済み ✓

#### コアメッセージシステム

- **メッセージレベル**: 5つの定義済みレベル（Debug、Info、Success、Warning、Error）と数値優先度（10、20、25、30、40）
- **カスタムレベル**: カスタム数値を持つユーザー定義のメッセージレベルをサポート
- **メッセージタグ**: スタイリングとフィルタリングのためのレベルベースのタグと追加のカスタムタグ
- **メッセージ作成**: メッセージ作成のための便利なメソッド（`Message::debug()`、`Message::info()`など）
- **メッセージ設定**: レベルタグをグローバルにカスタマイズするための`MessageConfig`

#### ストレージバックエンド

- **MemoryStorage**: テストと一時的なメッセージのためのスレッドセーフな`Arc<Mutex<VecDeque>>`を使用したインメモリストレージ
- **SessionStorage**: JSON シリアライゼーションを使用したセッションベースの永続ストレージ
  - カスタマイズ可能なセッションキー（デフォルト: `"_messages"`）
  - セッション可用性の検証
  - セッション統合のためのシリアライゼーション/デシリアライゼーション
- **CookieStorage**: 自動サイズ管理を備えたクッキーベースのストレージ
  - 設定可能なクッキー名とサイズ制限（デフォルト: 4KB）
  - サイズ制限を超える場合の二分探索を使用した自動メッセージ切り詰め
  - サイズ制限を超えた場合は古いメッセージから削除
- **FallbackStorage**: CookieとSessionストレージ間のインテリジェントなフォールバック
  - パフォーマンス向上のため最初にクッキーストレージを試行
  - クッキーサイズを超えた場合は自動的にセッションストレージにフォールバック
  - どのストレージバックエンドが使用されたかを追跡
  - 両方のバックエンドからのメッセージのフラッシュをサポート

#### ユーティリティ

- **二分探索アルゴリズム**: サイズ制限付きメッセージ管理の効率化
  - `bisect_keep_left()`: サイズ制限内で先頭から最大数のメッセージを保持
  - `bisect_keep_right()`: サイズ制限内で末尾から最大数のメッセージを保持
- **SafeData**: 事前にサニタイズされたHTMLコンテンツをレンダリングするためのHTML安全な文字列ラッパー
  - メッセージ内のHTMLの二重エスケープを防止
  - serdeサポートによるシリアライゼーション可能

#### ストレージトレイト

- **MessageStorage トレイト**: すべてのストレージバックエンドの統一インターフェース
  - `add()`: ストレージにメッセージを追加
  - `get_all()`: すべてのメッセージを取得してクリア
  - `peek()`: クリアせずにメッセージを表示
  - `clear()`: すべてのメッセージを削除

### 予定

#### ミドルウェア統合

- 自動メッセージ処理のためのリクエスト/レスポンスミドルウェア
- リクエストライフサイクル中の自動メッセージ取得と保存
- テンプレート統合のためのコンテキストプロセッサ

#### 高度な機能

- レベルによるメッセージフィルタリング
- メッセージ永続性制御（スティッキーメッセージ）
- メッセージの有効期限とTTLサポート
- 非同期ストレージバックエンドサポート
- カスタムシリアライゼーション形式（MessagePack、CBOR）
- 機密データのメッセージ暗号化
- メッセージ作成のレート制限

#### テンプレート統合

- メッセージレンダリングのためのテンプレートタグ
- Bootstrap/Tailwind CSSスタイリングを使用したデフォルトメッセージテンプレート
- クライアントサイドメッセージ表示のためのJavaScript統合
- トースト通知サポート
- メッセージ却下の追跡

#### テストユーティリティ

- テスト用のモックストレージバックエンド
- メッセージアサーションヘルパー
- 一般的なシナリオのテストフィクスチャ

## 使用方法

### 基本的なメッセージ作成

```rust
use reinhardt_messages::{Message, Level};

// レベルコンストラクタを使用
let msg = Message::new(Level::Info, "Operation completed");

// 便利なメソッドを使用
let debug_msg = Message::debug("Debug information");
let info_msg = Message::info("User logged in");
let success_msg = Message::success("Profile updated successfully");
let warning_msg = Message::warning("Disk space is low");
let error_msg = Message::error("Failed to connect to database");

// カスタムタグを使用
let tagged_msg = Message::info("Important notification")
    .with_tags(vec!["urgent".to_string(), "user-action".to_string()]);
```

### ストレージバックエンド

```rust
use reinhardt_messages::storage::{
    MessageStorage, MemoryStorage, SessionStorage,
    CookieStorage, FallbackStorage
};

// メモリストレージ（テスト用）
let mut memory = MemoryStorage::new();
memory.add(Message::info("Test message"));
let messages = memory.get_all();

// セッションストレージ
let mut session = SessionStorage::new()
    .with_session_key("custom_messages");
session.add(Message::success("Saved to session"));

// サイズ制限付きクッキーストレージ
let mut cookie = CookieStorage::new()
    .with_cookie_name("flash_messages")
    .with_max_size(2048);
cookie.add(Message::warning("Stored in cookie"));

// フォールバックストレージ（Cookie → Session）
let mut fallback = FallbackStorage::new()
    .with_max_cookie_size(4096);
fallback.add(Message::info("Automatically handled"));
fallback.store().unwrap(); // 必要に応じてフォールバックをトリガー
```

### カスタムメッセージレベル

```rust
use reinhardt_messages::{Level, MessageConfig};

// カスタムレベルを作成
let custom_level = Level::Custom(35);
let msg = Message::new(custom_level, "Custom priority message");

// カスタムレベルタグを設定
let mut config = MessageConfig::new();
config.set_tag(35, "urgent".to_string());
assert_eq!(config.get_tag(Level::Custom(35)), Some("urgent"));
```

### HTMLコンテンツのためのSafeData

```rust
use reinhardt_messages::SafeData;

// HTMLコンテンツを安全としてマーク
let safe_html = SafeData::new("<b>Bold text</b>");
println!("{}", safe_html); // 出力: <b>Bold text</b>

// Stringに変換
let html_string = safe_html.into_string();
```

## アーキテクチャ

### メッセージレベル

- 数値優先度システムにより、標準レベル間のカスタムレベルを許可
- レベルの順序: Debug (10) < Info (20) < Success (25) < Warning (30) < Error (40)
- カスタムレベルは細かい制御のために任意のi32値を持つことができる

### ストレージ戦略

- すべてのストレージバックエンドは一貫性のために`MessageStorage`トレイトを実装
- クッキーストレージは二分探索を使用してサイズ制限内に最大数のメッセージを効率的に収める
- フォールバックストレージはサイズ制約に基づいてメッセージを賢くルーティング
- セッションストレージは操作前にミドルウェアの可用性を検証

### サイズ管理

- 二分探索アルゴリズム（`bisect_keep_left`/`bisect_keep_right`）がメッセージ切り詰めを最適化
- 完全な再シリアライゼーションなしでの効率的なシリアライゼーションサイズ計算
- サイズ制限を超えた場合の自動的な古いものから削除

## テスト

Djangoのメッセージフレームワークテストに基づく包括的なテストカバレッジ:

- メッセージの作成と操作
- レベルの比較と順序付け
- すべてのストレージバックエンド操作
- サイズ制限の処理と切り詰め
- シリアライゼーション/デシリアライゼーション
- 二分探索アルゴリズム

## ライセンス

以下のいずれかのライセンスで利用可能です:

- Apache License, Version 2.0
- MIT license

お好みのライセンスをお選びください。
