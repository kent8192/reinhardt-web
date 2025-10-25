# reinhardt-signals

モデルライフサイクルイベント用のイベント駆動フック - Django signals の上位互換実装

## 概要

コンポーネント間の疎結合な通信のための型安全なシグナルシステム。モデル、マイグレーション、リクエスト、およびカスタムイベントのライフサイクルシグナルを提供します。ミドルウェア、シグナル合成、パフォーマンス監視などの高度な機能を備えた非同期および同期のシグナルディスパッチパターンをサポートします。

## 実装済み機能 ✓

### コアシグナルシステム

- **Signal**: 型安全なレシーバーを持つ汎用イベントディスパッチャー
- **SignalDispatcher**: すべてのシグナルディスパッチャー(非同期および同期)の共通トレイト
- **AsyncSignalDispatcher**: 非同期固有のメソッドを持つSignalDispatcherを拡張するトレイト
- **SignalError**: シグナル操作のエラー型
- **SignalRegistry**: シグナル管理のためのグローバルレジストリ

### シグナル接続とディスパッチ

- **基本接続**: シンプルなレシーバー登録のための`connect()`
- **オプション付き接続**: センダーフィルタリング、dispatch_uid、優先度を持つ`connect_with_options()`
- **完全オプション**: 述語を含む`connect_with_full_options()`
- **条件付きレシーバー**: 述語ベースの実行のための`connect_if()`
- **優先度ベースの実行**: 優先度でソートされるレシーバー(高いほど先に実行)
- **センダーフィルタリング**: TypeIdベースのセンダー型フィルタリング
- **dispatch_uid**: 一意の識別子による重複レシーバー登録の防止
- **切断**: dispatch_uidによるレシーバー削除のための`disconnect()`
- **全切断**: すべてのレシーバーをクリアするための`disconnect_all()`

### シグナルディスパッチメソッド

- **標準送信**: 通常のシグナルディスパッチのための`send()`
- **センダー付き送信**: センダー型フィルタリング付きの`send_with_sender()`
- **ロバスト送信**: 他のレシーバーを停止せずにエラーをキャッチする`send_robust()`
- **非同期送信**: ファイア・アンド・フォーゲットディスパッチのための`send_async()`

### シグナルミドルウェア

- **SignalMiddleware トレイト**: さまざまな段階でシグナルをインターセプトおよび変換
- **before_send**: シグナルがレシーバーに送信される前のフック
- **after_send**: シグナルがすべてのレシーバーに送信された後のフック
- **before_receiver**: レシーバーが実行される直前のフック
- **after_receiver**: レシーバーが実行された後のフック
- **ミドルウェアチェーン**: シグナルに複数のミドルウェアを追加可能
- **早期終了**: ミドルウェアがシグナル伝播を停止可能

### シグナル合成

- **チェーン**: `chain()`で順番にシグナルを接続
- **変換付きチェーン**: シグナル間でデータを変換する`chain_with()`
- **マージ**: 複数のシグナルを1つに結合する`Signal::merge()`
- **フィルター**: 述語に基づいてフィルタされたシグナルを作成する`filter()`
- **マップ**: 関数を通じてシグナル値を変換する`map()`

### テストユーティリティ

- **SignalSpy**: シグナル呼び出しを記録およびアサートするテストユーティリティ
  - `call_count()`: シグナルが送信された回数
  - `was_called()`: シグナルが呼び出されたかチェック
  - `was_called_with_count()`: 正確な呼び出し回数をチェック
  - `total_receivers_called()`: レシーバー実行の合計数
  - `has_errors()`: エラーがあるかチェック
  - `errors()`: すべてのエラーメッセージを取得
  - `reset()`: 記録された呼び出しをクリア
  - `instances()`: すべての送信されたインスタンスを取得
  - `last_instance()`: 最後に送信されたインスタンスを取得

### パフォーマンス監視

- **SignalMetrics**: パフォーマンスメトリクス収集
  - `send_count`: 送信されたシグナルの総数
  - `receiver_executions`: レシーバー実行の総数
  - `failed_executions`: 失敗した実行の数
  - `success_rate()`: 成功率のパーセンテージ
  - `avg_execution_time()`: レシーバーの平均実行時間
  - `min_execution_time()`: 最小実行時間
  - `max_execution_time()`: 最大実行時間
- **ゼロコスト**: メトリクスは最小限のオーバーヘッドでアトミック操作を使用
- **スレッドセーフ**: 並行メトリクス収集
- **リセット可能**: テストと監視のための`reset_metrics()`

### シグナルコンテキスト

- **SignalContext**: シグナルと一緒にメタデータを渡す
  - `insert()`: コンテキスト値を追加
  - `get()`: コンテキスト値を取得
  - `contains_key()`: キーの存在をチェック
  - `remove()`: コンテキスト値を削除
  - `clear()`: すべてのコンテキストデータをクリア
  - `keys()`: すべてのコンテキストキーを取得

### 組み込みモデルライフサイクルシグナル

- **pre_save**: モデルインスタンスを保存する前
- **post_save**: モデルインスタンスを保存した後
- **pre_delete**: モデルインスタンスを削除する前
- **post_delete**: モデルインスタンスを削除した後
- **pre_init**: モデル初期化の開始時(`PreInitEvent`を含む)
- **post_init**: モデル初期化の終了時(`PostInitEvent`を含む)
- **m2m_changed**: 多対多リレーションシップが変更されたとき
  - アクション型と関連オブジェクトを含む`M2MChangeEvent`を含む
  - `M2MAction`列挙型をサポート: PreAdd, PostAdd, PreRemove, PostRemove, PreClear, PostClear

### 組み込みマイグレーションシグナル

- **pre_migrate**: マイグレーション実行前(`MigrationEvent`を含む)
- **post_migrate**: マイグレーション実行後(`MigrationEvent`を含む)

### 組み込みリクエストシグナル

- **request_started**: HTTPリクエスト開始時(`RequestStartedEvent`を含む)
- **request_finished**: HTTPリクエスト終了時(`RequestFinishedEvent`を含む)
- **got_request_exception**: リクエスト処理中に例外が発生したとき(`GotRequestExceptionEvent`を含む)

### 組み込み管理シグナル

- **setting_changed**: 設定が変更されたとき(`SettingChangedEvent`を含む)
- **class_prepared**: モデルクラスが準備されたとき(`ClassPreparedEvent`を含む)

### データベースライフサイクルイベント (SQLAlchemy スタイル)

モジュール: `db_events`

- **before_insert**: レコード挿入前
- **after_insert**: レコード挿入後
- **before_update**: レコード更新前
- **after_update**: レコード更新後
- **before_delete**: レコード削除前
- **after_delete**: レコード削除後
- **DbEvent**: テーブル、ID、データフィールドを持つ汎用データベースイベント構造

### 同期シグナルサポート

モジュール: `dispatch`

- **SyncSignal**: Django スタイルの同期シグナルディスパッチャー
- **弱参照**: 自動クリーンアップのための弱いレシーバー参照をサポート
- **use_caching**: パフォーマンス向上のためのオプションのキャッシング
- **互換API**: Django の Signal クラスインターフェースを模倣

### 開発者の利便性

- **connect_receiver! マクロ**: すべての接続オプションをサポートする簡素化されたレシーバー接続構文

## Rust 固有の拡張機能 ✓

- **コンパイル時型安全性**: TypeIdベースのセンダーフィルタリングがコンパイル時にエラーをキャッチ
- **ゼロコスト抽象化**: アトミックメトリクスを持つ効率的なArcベースのレシーバーストレージ
- **メモリ安全性**: Rustの所有権システムによる自動クリーンアップ
- **Async/Await ネイティブ**: 効率的な非同期実行のためにTokio上に構築
- **人間工学的マクロ**: よりクリーンな構文のための`connect_receiver!`マクロ
- **スレッド安全性**: 並行レシーバーアクセスのためのRwLock
- **パフォーマンス監視**: アトミック操作による組み込みメトリクス

## 予定機能

### 高度な機能

- **シグナルバッチング**: 複数のシグナルを単一のディスパッチにバッチ処理
- **シグナルスロットリング**: シグナル発行のレート制限
- **永続シグナル**: 永続ストレージからシグナルを保存および再生
- **シグナルリプレイ**: デバッグとテストのための過去のシグナルの再生
- **シグナル履歴**: タイムスタンプ付きのシグナル発行履歴の追跡
- **デッドレターキュー**: 再試行ロジックによる失敗したシグナルの処理

### 統合機能

- **ORM統合**: ORM操作からの自動シグナルディスパッチ
- **トランザクションサポート**: データベーストランザクションライフサイクルに結び付けられたシグナル
- **分散シグナル**: メッセージブローカー経由のクロスサービスシグナルディスパッチ
- **WebSocketシグナル**: クライアントへのリアルタイムシグナル伝播
- **GraphQLサブスクリプション**: シグナルベースのGraphQLサブスクリプションサポート

### 開発者ツール

- **シグナルデバッガー**: シグナルフローのビジュアルデバッグツール
- **シグナルプロファイラー**: シグナルシステムのパフォーマンスプロファイリング
- **シグナルドキュメント生成器**: シグナル定義からドキュメントを自動生成
- **シグナル可視化**: シグナル接続のグラフィカル表現

## 使用例

## 基本的なシグナル接続

```rust
use reinhardt_signals::{post_save, Signal, SignalError};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct User {
    id: i32,
    name: String,
}

// post_save シグナルにレシーバーを接続
post_save::<User>().connect(|instance: Arc<User>| async move {
    println!("User saved: {:?}", instance);
    Ok(())
});

// シグナルを送信
let user = User { id: 1, name: "Alice".to_string() };
post_save::<User>().send(user).await?;
```

## センダーフィルタリング

```rust
use std::any::TypeId;

struct BlogPost;
struct ForumPost;

// BlogPost シグナルのみをリッスンするレシーバーを接続
post_save::<Post>().connect_with_options(
    |instance: Arc<Post>| async move {
        println!("Blog post saved!");
        Ok(())
    },
    Some(TypeId::of::<BlogPost>()),  // BlogPost のみトリガー
    None,
);

// これはレシーバーをトリガーします
post_save::<Post>()
    .send_with_sender(post, Some(TypeId::of::<BlogPost>()))
    .await?;

// これはレシーバーをトリガーしません
post_save::<Post>()
    .send_with_sender(post, Some(TypeId::of::<ForumPost>()))
    .await?;
```

## dispatch_uid による重複登録の防止

```rust
use reinhardt_signals::connect_receiver;

// 最初の登録
connect_receiver!(
    post_save::<User>(),
    |instance| async move { Ok(()) },
    dispatch_uid = "my_unique_handler"
);

// これは最初の登録を置き換えます(重複しません)
connect_receiver!(
    post_save::<User>(),
    |instance| async move { Ok(()) },
    dispatch_uid = "my_unique_handler"
);
```

## ロバストなエラーハンドリング

```rust
// シグナルをロバストに送信 - レシーバーが失敗しても続行
let results = post_save::<User>().send_robust(user, None).await;

for result in results {
    match result {
        Ok(_) => println!("Receiver succeeded"),
        Err(e) => eprintln!("Receiver failed: {}", e),
    }
}
```

## connect_receiver! マクロの使用

```rust
use reinhardt_signals::{connect_receiver, post_save};

// シンプルな接続
connect_receiver!(post_save::<User>(), my_receiver);

// dispatch_uid 付き
connect_receiver!(
    post_save::<User>(),
    my_receiver,
    dispatch_uid = "unique_id"
);

// センダーフィルタリング付き
connect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = BlogPost
);

// 両方付き
connect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = BlogPost,
    dispatch_uid = "blog_handler"
);
```

## 優先度ベースの実行

```rust
use reinhardt_signals::{connect_receiver, post_save};

// 高優先度レシーバーが最初に実行されます
connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("Critical: Log to audit system");
        Ok(())
    },
    priority = 100  // 最初に実行
);

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("Normal: Send notification email");
        Ok(())
    },
    priority = 50  // 2番目に実行
);

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("Low: Update cache");
        Ok(())
    },
    priority = 10  // 最後に実行
);

// 優先度を他のオプションと組み合わせることができます
connect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = AdminUser,
    priority = 200,
    dispatch_uid = "admin_handler"
);
```

## 条件付きレシーバー (述語)

```rust
use reinhardt_signals::post_save;

// 管理者ロールを持つユーザーのみ実行
post_save::<User>().connect_if(
    |instance| async move {
        println!("Admin user saved: {:?}", instance.name);
        Ok(())
    },
    |user| user.is_admin  // 述語 - true の場合のみ実行
);

// アクティブユーザーのみ実行
post_save::<User>().connect_if(
    |instance| async move {
        send_welcome_email(&instance).await?;
        Ok(())
    },
    |user| user.is_active
);

// 複雑な条件
post_save::<User>().connect_if(
    |instance| async move {
        alert_security_team(&instance).await?;
        Ok(())
    },
    |user| user.login_attempts > 5 && !user.is_locked
);

// 優先度および他のオプションと組み合わせる
signal.connect_with_full_options(
    |instance| async move {
        process_premium_user(&instance).await?;
        Ok(())
    },
    None,  // sender_type_id
    Some("premium_handler".to_string()),  // dispatch_uid
    100,  // priority
    Some(|user: &User| user.is_premium),  // predicate
);
```

## シグナルミドルウェア

ミドルウェアを使用すると、さまざまな段階でシグナルの動作をインターセプトおよび変更できます:

```rust
use reinhardt_signals::{Signal, SignalMiddleware, SignalError};
use std::sync::Arc;

// ロギングミドルウェアを作成
struct LoggingMiddleware;

#[async_trait::async_trait]
impl SignalMiddleware<User> for LoggingMiddleware {
    async fn before_send(&self, instance: &User) -> Result<bool, SignalError> {
        println!("Signal about to be sent for user: {}", instance.id);
        Ok(true) // シグナル伝播を停止するには false を返す
    }

    async fn after_send(&self, instance: &User, results: &[Result<(), SignalError>]) -> Result<(), SignalError> {
        println!("Signal sent. {} receivers executed", results.len());
        Ok(())
    }

    async fn before_receiver(&self, instance: &User, dispatch_uid: Option<&str>) -> Result<bool, SignalError> {
        println!("Receiver {:?} about to execute", dispatch_uid);
        Ok(true) // このレシーバーをスキップするには false を返す
    }

    async fn after_receiver(&self, instance: &User, dispatch_uid: Option<&str>, result: &Result<(), SignalError>) -> Result<(), SignalError> {
        if result.is_err() {
            println!("Receiver {:?} failed", dispatch_uid);
        }
        Ok(())
    }
}

// シグナルにミドルウェアを追加
let signal = post_save::<User>();
signal.add_middleware(LoggingMiddleware);

// 認証/認可のためのミドルウェアを作成
struct AuthMiddleware {
    required_role: String,
}

#[async_trait::async_trait]
impl SignalMiddleware<User> for AuthMiddleware {
    async fn before_send(&self, instance: &User) -> Result<bool, SignalError> {
        if !instance.has_role(&self.required_role) {
            return Ok(false); // 必要なロールを持たない場合はシグナルをブロック
        }
        Ok(true)
    }
}
```

## SignalSpy によるテスト

`SignalSpy`は、アサーションのためにシグナル呼び出しを記録するテストユーティリティです:

```rust
use reinhardt_signals::{Signal, SignalSpy};

#[tokio::test]
async fn test_user_creation() {
    let signal = post_save::<User>();
    let spy = SignalSpy::new();

    // スパイをミドルウェアとしてアタッチ
    signal.add_middleware(spy.clone());

    // レシーバーを接続
    signal.connect(|user| async move {
        send_welcome_email(&user).await?;
        Ok(())
    });

    // アクションを実行
    let user = User::new("Alice");
    signal.send(user).await.unwrap();

    // シグナルが呼び出されたことをアサート
    assert!(spy.was_called());
    assert_eq!(spy.call_count(), 1);
    assert_eq!(spy.total_receivers_called(), 1);
    assert!(!spy.has_errors());
}

#[tokio::test]
async fn test_error_handling() {
    let signal = post_save::<User>();
    let spy = SignalSpy::new();
    signal.add_middleware(spy.clone());

    // 失敗する可能性のあるレシーバー
    signal.connect(|user| async move {
        if user.email.is_empty() {
            return Err(SignalError::new("Email required"));
        }
        Ok(())
    });

    let user = User { email: String::new(), ..Default::default() };
    let _ = signal.send_robust(user, None).await;

    // エラーをチェック
    assert!(spy.has_errors());
    let errors = spy.errors();
    assert_eq!(errors[0], "Email required");
}
```

## カスタムシグナル

```rust
use reinhardt_signals::Signal;

// カスタムシグナルを定義
let payment_completed = Signal::<PaymentInfo>::new("payment_completed");

// レシーバーを接続
payment_completed.connect(|info| async move {
    println!("Payment completed: ${}", info.amount);
    Ok(())
});

// シグナルを送信
payment_completed.send(payment_info).await?;
```

## レシーバーの切断

```rust
let signal = post_save::<User>();

// dispatch_uid 付きで接続
connect_receiver!(
    signal,
    my_receiver,
    dispatch_uid = "removable_handler"
);

// 後で切断
signal.disconnect("removable_handler");
```

## 組み込みシグナルタイプ

Reinhardt は、さまざまなフレームワークイベント用の包括的なシグナルタイプセットを提供します:

## モデルライフサイクルシグナル

```rust
use reinhardt_signals::{pre_init, post_init, pre_save, post_save, pre_delete, post_delete, PreInitEvent, PostInitEvent};

// Pre-init: モデル初期化前に呼び出される
pre_init::<User>().connect(|event| async move {
    println!("Initializing model: {}", event.model_type);
    Ok(())
});

// Post-init: モデル初期化後に呼び出される
post_init::<User>().connect(|event| async move {
    println!("User initialized: {:?}", event.instance);
    Ok(())
});

// モデル保存/削除シグナル
pre_save::<User>().connect(|user| async move { Ok(()) });
post_save::<User>().connect(|user| async move { Ok(()) });
pre_delete::<User>().connect(|user| async move { Ok(()) });
post_delete::<User>().connect(|user| async move { Ok(()) });
```

## 多対多リレーションシップシグナル

```rust
use reinhardt_signals::{m2m_changed, M2MAction, M2MChangeEvent};

m2m_changed::<User, Group>().connect(|event| async move {
    match event.action {
        M2MAction::PostAdd => println!("Added {} groups to user", event.related.len()),
        M2MAction::PostRemove => println!("Removed {} groups from user", event.related.len()),
        M2MAction::PostClear => println!("Cleared all groups from user"),
        _ => {}
    }
    Ok(())
});

// m2m_changed シグナルの送信
let event = M2MChangeEvent::new(user, M2MAction::PostAdd, vec![group1, group2])
    .with_reverse(false)
    .with_model_name("Group");
m2m_changed::<User, Group>().send(event).await?;
```

## マイグレーションシグナル

```rust
use reinhardt_signals::{pre_migrate, post_migrate, MigrationEvent};

// Pre-migrate: マイグレーション実行前
pre_migrate().connect(|event| async move {
    println!("Running migration {} for app {}", event.migration_name, event.app_name);
    Ok(())
});

// Post-migrate: マイグレーション実行後
post_migrate().connect(|event| async move {
    println!("Completed migration: {}", event.migration_name);
    Ok(())
});

// マイグレーションシグナルの送信
let event = MigrationEvent::new("myapp", "0001_initial")
    .with_plan(vec!["CreateModel".to_string()]);
pre_migrate().send(event).await?;
```

## リクエスト処理シグナル

```rust
use reinhardt_signals::{request_started, request_finished, got_request_exception};
use reinhardt_signals::{RequestStartedEvent, RequestFinishedEvent, GotRequestExceptionEvent};

// リクエスト開始
request_started().connect(|event| async move {
    println!("Request started: {:?}", event.environ);
    Ok(())
});

// リクエスト終了
request_finished().connect(|event| async move {
    println!("Request completed");
    Ok(())
});

// 例外処理
got_request_exception().connect(|event| async move {
    eprintln!("Request error: {}", event.error_message);
    Ok(())
});
```

## 管理シグナル

```rust
use reinhardt_signals::{setting_changed, class_prepared};
use reinhardt_signals::{SettingChangedEvent, ClassPreparedEvent};

// 設定変更
setting_changed().connect(|event| async move {
    println!("Setting {} changed from {:?} to {}",
        event.setting_name, event.old_value, event.new_value);
    Ok(())
});

// クラス準備
class_prepared().connect(|event| async move {
    println!("Model {} prepared for app {}", event.model_name, event.app_label);
    Ok(())
});
```

## シグナル合成

Reinhardt シグナルは、複雑なイベントフローを構築するための強力な合成パターンをサポートします:

## シグナルのチェーン

```rust
use reinhardt_signals::Signal;

let user_created = Signal::<User>::new("user_created");
let send_welcome_email = Signal::<User>::new("send_welcome_email");

// シグナルをチェーン - user_created が送信されると、send_welcome_email が自動的にトリガーされます
user_created.chain(&send_welcome_email);

send_welcome_email.connect(|user| async move {
    email_service.send_welcome(&user).await?;
    Ok(())
});

// user_created への送信は両方のシグナルをトリガーします
user_created.send(new_user).await?;
```

## 変換付きチェーン

```rust
let user_created = Signal::<User>::new("user_created");
let send_notification = Signal::<Notification>::new("send_notification");

// チェーン時に User を Notification に変換
user_created.chain_with(&send_notification, |user: Arc<User>| {
    Notification {
        user_id: user.id,
        message: format!("Welcome, {}!", user.name),
        priority: Priority::High,
    }
});
```

## 複数シグナルのマージ

```rust
let user_login = Signal::<User>::new("user_login");
let user_signup = Signal::<User>::new("user_signup");
let password_reset = Signal::<User>::new("password_reset");

// 複数のシグナルを1つにマージ
let any_user_activity = Signal::merge(vec![&user_login, &user_signup, &password_reset]);

// このレシーバーは3つのイベントのいずれかでトリガーされます
any_user_activity.connect(|user| async move {
    update_last_activity(&user).await?;
    Ok(())
});
```

## シグナル発行のフィルタリング

```rust
let user_signal = Signal::<User>::new("user_changes");

// 管理者ユーザーのみトリガーするフィルタされたシグナルを作成
let admin_signal = user_signal.filter(|user| user.is_admin);

admin_signal.connect(|admin_user| async move {
    log_admin_action(&admin_user).await?;
    Ok(())
});

// 管理者ユーザーのみがフィルタされたシグナルをトリガーします
user_signal.send(regular_user).await?; // admin_signal をトリガーしません
user_signal.send(admin_user).await?;   // admin_signal をトリガーします
```

## シグナル値のマッピング

```rust
let user_signal = Signal::<User>::new("user_signal");

// User をユーザーIDにマップ
let user_id_signal: Signal<i32> = user_signal.map(|user: Arc<User>| user.id);

user_id_signal.connect(|user_id| async move {
    println!("User ID: {}", user_id);
    Ok(())
});
```

## 複雑な合成

複数の合成演算子を組み合わせて洗練されたイベントフローを実現:

```rust
let user_signal = Signal::<User>::new("users");

// 管理者ユーザーをフィルタし、そのIDにマップ
let admin_ids: Signal<i32> = user_signal
    .filter(|user| user.is_admin)
    .map(|user: Arc<User>| user.id);

admin_ids.connect(|admin_id| async move {
    audit_log.record_admin_activity(*admin_id).await?;
    Ok(())
});
```

## パフォーマンスメトリクス

組み込みメトリクス収集でシグナルパフォーマンスを監視:

```rust
let signal = Signal::<User>::new("user_updates");

signal.connect(|user| async move {
    process_user(&user).await?;
    Ok(())
});

// いくつかのシグナルを送信
for i in 0..100 {
    signal.send(create_user(i)).await?;
}

// メトリクスを取得
let metrics = signal.metrics();
println!("Signals sent: {}", metrics.send_count);
println!("Receivers executed: {}", metrics.receiver_executions);
println!("Success rate: {:.2}%", metrics.success_rate());
println!("Avg execution time: {:?}", metrics.avg_execution_time());
println!("Min execution time: {:?}", metrics.min_execution_time());
println!("Max execution time: {:?}", metrics.max_execution_time());

// メトリクスをリセット
signal.reset_metrics();
```

**利用可能なメトリクス:**

- `send_count` - シグナルが送信された総回数
- `receiver_executions` - レシーバー実行の総数
- `failed_executions` - 失敗したレシーバー実行の数
- `success_rate()` - パーセンテージ(0-100)での成功率
- `avg_execution_time()` - レシーバーの平均実行時間
- `min_execution_time()` - レシーバーの最小実行時間
- `max_execution_time()` - レシーバーの最大実行時間

**機能:**

- アクセスされない場合はゼロコスト
- スレッドセーフなアトミック操作
- クローンされたシグナル間で共有
- テストと監視のためのリセット可能

## Django vs Reinhardt シグナル比較

| 機能                | Django | Reinhardt | 注記                                               |
|---------------------|--------|-----------|----------------------------------------------------|
| センダーフィルタリング | ✅      | ✅         | Rust は型安全フィルタリングのために TypeId を使用 |
| dispatch_uid        | ✅      | ✅         | 重複登録を防止                                     |
| send_robust         | ✅      | ✅         | レシーバーが失敗しても実行を継続                   |
| 弱参照              | ✅      | ✅         | dispatch モジュールで利用可能                      |
| @receiver デコレータ| ✅      | ✅         | `connect_receiver!` マクロを使用                   |
| 非同期サポート      | ⚠️     | ✅         | ネイティブ async/await サポート                    |
| 型安全性            | ❌      | ✅         | コンパイル時型チェック                             |
| メモリ安全性        | ⚠️     | ✅         | Rust 所有権により保証                              |
| ミドルウェア        | ❌      | ✅         | 複数段階でシグナルをインターセプト                 |
| シグナル合成        | ❌      | ✅         | チェーン、マージ、フィルター、マップシグナル       |
| パフォーマンスメトリクス | ❌  | ✅         | 組み込みパフォーマンス監視                         |
| 述語                | ❌      | ✅         | 条件付きレシーバー実行                             |
| 優先度順序          | ❌      | ✅         | 優先度順にレシーバーを実行                         |

## パフォーマンス

Reinhardt シグナルは高性能向けに設計されています:

- **Arc ベースストレージ**: 最小限のオーバーヘッドでレシーバーの効率的なクローニング
- **並行性のための RwLock**: 複数のリーダー、単一のライターで最適なスループット
- **ゼロアロケーション**: センダーフィルタリング用(TypeId 比較)
- **アトミックメトリクス**: ロックフリーパフォーマンス監視
- **非同期ランタイム**: 効率的な非同期実行のために Tokio を活用
- **ヒープアロケーションなし**: シンプルなシグナルディスパッチパスの場合

## Django からの移行

```python
# Django
from django.db.models.signals import post_save
from django.dispatch import receiver

@receiver(post_save, sender=User)
def on_user_saved(sender, instance, created, **kwargs):
    print(f"User saved: {instance}")
```

```rust
// Reinhardt
use reinhardt_signals::{connect_receiver, post_save};

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("User saved: {:?}", instance);
        Ok(())
    },
    sender = UserModel
);
```

## ライセンス

このクレートは Reinhardt プロジェクトの一部であり、同じライセンス条項に従います。