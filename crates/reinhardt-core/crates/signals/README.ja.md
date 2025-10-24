# reinhardt-signals

モデルライフサイクルイベント用のイベント駆動フック - Django signals の上位互換実装

## 概要

コンポーネント間の疎結合な通信のための型安全なシグナルシステムです。モデル操作用のpre_save、post_save、pre_delete、post_delete、m2m_changedシグナルを提供します。アプリケーション全体でカスタムシグナルを定義してディスパッチできます。

## 機能

## ✅ Django互換機能

- **非同期/同期シグナル**: 非同期と同期の両方のシグナルハンドリングを完全サポート
- **送信者フィルタリング**: 特定の送信者からのシグナルのみにレシーバーを接続
- **dispatch_uid**: レシーバーの重複登録を防止
- **send_robust**: エラーを捕捉し、他のレシーバーを停止せずに処理を継続
- **グローバルレジストリ**: 自動管理される型安全なシグナルレジストリ
- **組み込みシグナル**: pre_save、post_save、pre_delete、post_delete、m2m_changed、pre_migrate、post_migrate

## 🚀 Rust固有の拡張機能

- **コンパイル時型安全性**: TypeIdベースの送信者フィルタリングでコンパイル時にエラーを検出
- **ゼロコスト抽象化**: Arcベースの効率的なレシーバー格納
- **メモリ安全性**: Rustの所有権システムによる自動クリーンアップ
- **人間工学的マクロ**: より簡潔な構文のための`connect_receiver!`マクロ

## 使用例

## 基本的なシグナル接続

```rustuse reinhardt_signals::{post_save, Signal, SignalError};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct User {
    id: i32,
    name: String,
}

// post_saveシグナルにレシーバーを接続post_save::<User>().connect(|instance: Arc<User>| async move {
    println!("ユーザー保存: {:?}", instance);
    Ok(())
});

// シグナルを送信let user = User { id: 1, name: "Alice".to_string() };
post_save::<User>().send(user).await?;
```

## 送信者フィルタリング

```rustuse std::any::TypeId;

struct BlogPost;struct ForumPost;

// BlogPostシグナルのみを受信するレシーバーを接続post_save::<Post>().connect_with_options(
    |instance: Arc<Post>| async move {
        println!("ブログ記事が保存されました！");
        Ok(())
    },
    Some(TypeId::of::<BlogPost>()),  // BlogPostのみトリガー
    None,
);

// これはレシーバーをトリガーしますpost_save::<Post>()
    .send_with_sender(post, Some(TypeId::of::<BlogPost>()))
    .await?;

// これはレシーバーをトリガーしませんpost_save::<Post>()
    .send_with_sender(post, Some(TypeId::of::<ForumPost>()))
    .await?;
```

## dispatch_uidで重複登録を防止

```rustuse reinhardt_signals::connect_receiver;

// 最初の登録connect_receiver!(
    post_save::<User>(),
    |instance| async move { Ok(()) },
    dispatch_uid = "my_unique_handler"
);

// これは最初の登録を置き換えます（重複しません）connect_receiver!(
    post_save::<User>(),
    |instance| async move { Ok(()) },
    dispatch_uid = "my_unique_handler"
);
```

## 堅牢なエラーハンドリング

```rust
// シグナルを堅牢に送信 - レシーバーが失敗しても継続let results = post_save::<User>().send_robust(user, None).await;

for result in results {
    match result {
        Ok(_) => println!("レシーバー成功"),
        Err(e) => eprintln!("レシーバー失敗: {}", e),
    }
}
```

## connect_receiver!マクロの使用

```rustuse reinhardt_signals::{connect_receiver, post_save};

// シンプルな接続connect_receiver!(post_save::<User>(), my_receiver);

// dispatch_uidと一緒にconnect_receiver!(
    post_save::<User>(),
    my_receiver,
    dispatch_uid = "unique_id"
);

// 送信者フィルタリングと一緒にconnect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = BlogPost
);

// 両方と一緒にconnect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = BlogPost,
    dispatch_uid = "blog_handler"
);
```

## 優先度ベースの実行

```rustuse reinhardt_signals::{connect_receiver, post_save};

// 優先度の高いレシーバーが先に実行されますconnect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("重要: 監査システムにログ記録");
        Ok(())
    },
    priority = 100  // 最初に実行
);

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("通常: 通知メール送信");
        Ok(())
    },
    priority = 50  // 2番目に実行
);

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("低優先度: キャッシュ更新");
        Ok(())
    },
    priority = 10  // 最後に実行
);

// 優先度を他のオプションと組み合わせることができますconnect_receiver!(
    post_save::<User>(),
    my_receiver,
    sender = AdminUser,
    priority = 200,
    dispatch_uid = "admin_handler"
);
```

## 条件付きレシーバー（述語）

```rustuse reinhardt_signals::post_save;

// 管理者ユーザーの場合のみ実行post_save::<User>().connect_if(
    |instance| async move {
        println!("管理者ユーザー保存: {:?}", instance.name);
        Ok(())
    },
    |user| user.is_admin  // 述語 - trueの場合のみ実行
);

// アクティブユーザーの場合のみ実行post_save::<User>().connect_if(
    |instance| async move {
        send_welcome_email(&instance).await?;
        Ok(())
    },
    |user| user.is_active
);

// 複雑な条件post_save::<User>().connect_if(
    |instance| async move {
        alert_security_team(&instance).await?;
        Ok(())
    },
    |user| user.login_attempts > 5 && !user.is_locked
);

// 優先度やその他のオプションと組み合わせsignal.connect_with_full_options(
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

ミドルウェアを使用すると、様々な段階でシグナルの動作をインターセプトおよび変更できます：

```rustuse reinhardt_signals::{Signal, SignalMiddleware, SignalError};
use std::sync::Arc;

// ロギングミドルウェアを作成struct LoggingMiddleware;

#[async_trait::async_trait]
impl SignalMiddleware<User> for LoggingMiddleware {
    async fn before_send(&self, instance: &User) -> Result<bool, SignalError> {
        println!("シグナルが送信されようとしています: ユーザーID {}", instance.id);
        Ok(true) // falseを返すとシグナルの伝播を停止
    }

    async fn after_send(&self, instance: &User, results: &[Result<(), SignalError>]) -> Result<(), SignalError> {
        println!("シグナルが送信されました。{}個のレシーバーが実行されました", results.len());
        Ok(())
    }

    async fn before_receiver(&self, instance: &User, dispatch_uid: Option<&str>) -> Result<bool, SignalError> {
        println!("レシーバー {:?} が実行されようとしています", dispatch_uid);
        Ok(true) // falseを返すとこのレシーバーをスキップ
    }

    async fn after_receiver(&self, instance: &User, dispatch_uid: Option<&str>, result: &Result<(), SignalError>) -> Result<(), SignalError> {
        if result.is_err() {
            println!("レシーバー {:?} が失敗しました", dispatch_uid);
        }
        Ok(())
    }
}

// シグナルにミドルウェアを追加let signal = post_save::<User>();
signal.add_middleware(LoggingMiddleware);

// 認証/認可用のミドルウェアを作成struct AuthMiddleware {
    required_role: String,
}

#[async_trait::async_trait]
impl SignalMiddleware<User> for AuthMiddleware {
    async fn before_send(&self, instance: &User) -> Result<bool, SignalError> {
        if !instance.has_role(&self.required_role) {
            return Ok(false); // ユーザーが必要なロールを持っていない場合はシグナルをブロック
        }
        Ok(true)
    }
}
```

## SignalSpyを使用したテスト

`SignalSpy`はシグナル呼び出しを記録してアサーションするためのテストユーティリティです：

```rustuse reinhardt_signals::{Signal, SignalSpy};

#[tokio::test]
async fn test_user_creation() {
    let signal = post_save::<User>();
    let spy = SignalSpy::new();

    // スパイをミドルウェアとして接続
    signal.add_middleware(spy.clone());

    // レシーバーを接続
    signal.connect(|user| async move {
        send_welcome_email(&user).await?;
        Ok(())
    });

    // アクションを実行
    let user = User::new("Alice");
    signal.send(user).await.unwrap();

    // シグナルが呼ばれたことをアサート
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
            return Err(SignalError::new("メールアドレスが必要です"));
        }
        Ok(())
    });

    let user = User { email: String::new(), ..Default::default() };
    let _ = signal.send_robust(user, None).await;

    // エラーをチェック
    assert!(spy.has_errors());
    let errors = spy.errors();
    assert_eq!(errors[0], "メールアドレスが必要です");
}
```

## 組み込みシグナルタイプ

Reinhardtは、さまざまなフレームワークイベント用の包括的なシグナルタイプセットを提供します：

## モデルライフサイクルシグナル

```rustuse reinhardt_signals::{pre_init, post_init, pre_save, post_save, pre_delete, post_delete, PreInitEvent, PostInitEvent};

// Pre-init: モデル初期化前に呼び出されるpre_init::<User>().connect(|event| async move {
    println!("モデルを初期化中: {}", event.model_type);
    Ok(())
});

// Post-init: モデル初期化後に呼び出されるpost_init::<User>().connect(|event| async move {
    println!("ユーザーが初期化されました: {:?}", event.instance);
    Ok(())
});

// モデル保存/削除シグナルpre_save::<User>().connect(|user| async move { Ok(()) });
post_save::<User>().connect(|user| async move { Ok(()) });pre_delete::<User>().connect(|user| async move { Ok(()) });
post_delete::<User>().connect(|user| async move { Ok(()) });
```

## 多対多リレーションシップシグナル

```rustuse reinhardt_signals::{m2m_changed, M2MAction, M2MChangeEvent};

m2m_changed::<User, Group>().connect(|event| async move {
    match event.action {
        M2MAction::PostAdd => println!("ユーザーに{}個のグループを追加", event.related.len()),
        M2MAction::PostRemove => println!("ユーザーから{}個のグループを削除", event.related.len()),
        M2MAction::PostClear => println!("ユーザーからすべてのグループをクリア"),
        _ => {}
    }
    Ok(())
});

// m2m_changedシグナルの送信let event = M2MChangeEvent::new(user, M2MAction::PostAdd, vec![group1, group2])
    .with_reverse(false)
    .with_model_name("Group");m2m_changed::<User, Group>().send(event).await?;
```

## マイグレーションシグナル

```rustuse reinhardt_signals::{pre_migrate, post_migrate, MigrationEvent};

// Pre-migrate: マイグレーション実行前pre_migrate().connect(|event| async move {
    println!("アプリ{}のマイグレーション{}を実行中", event.app_name, event.migration_name);
    Ok(())
});

// Post-migrate: マイグレーション実行後post_migrate().connect(|event| async move {
    println!("マイグレーション完了: {}", event.migration_name);
    Ok(())
});

// マイグレーションシグナルの送信let event = MigrationEvent::new("myapp", "0001_initial")
    .with_plan(vec!["CreateModel".to_string()]);pre_migrate().send(event).await?;
```

## リクエスト処理シグナル

```rustuse reinhardt_signals::{request_started, request_finished, got_request_exception};
use reinhardt_signals::{RequestStartedEvent, RequestFinishedEvent, GotRequestExceptionEvent};

// リクエスト開始request_started().connect(|event| async move {
    println!("リクエスト開始: {:?}", event.environ);
    Ok(())
});

// リクエスト完了request_finished().connect(|event| async move {
    println!("リクエスト完了");
    Ok(())
});

// 例外処理got_request_exception().connect(|event| async move {
    eprintln!("リクエストエラー: {}", event.error_message);
    Ok(())
});
```

## 管理シグナル

```rustuse reinhardt_signals::{setting_changed, class_prepared};
use reinhardt_signals::{SettingChangedEvent, ClassPreparedEvent};

// 設定変更setting_changed().connect(|event| async move {
    println!("設定{}が{:?}から{}に変更されました",
        event.setting_name, event.old_value, event.new_value);
    Ok(())
});

// クラス準備完了class_prepared().connect(|event| async move {
    println!("アプリ{}のモデル{}が準備されました", event.app_label, event.model_name);
    Ok(())
});
```

## シグナルコンポジション

Reinhardtシグナルは、複雑なイベントフローを構築するための強力な合成パターンをサポートしています：

## シグナルのチェーン

```rustuse reinhardt_signals::Signal;

let user_created = Signal::<User>::new("user_created");let send_welcome_email = Signal::<User>::new("send_welcome_email");

// シグナルをチェーン - user_createdが送信されると、send_welcome_emailが自動的にトリガーされるuser_created.chain(&send_welcome_email);

send_welcome_email.connect(|user| async move {
    email_service.send_welcome(&user).await?;
    Ok(())
});

// user_createdへの送信は両方のシグナルをトリガーしますuser_created.send(new_user).await?;
```

## 変換を伴うチェーン

```rustlet user_created = Signal::<User>::new("user_created");
let send_notification = Signal::<Notification>::new("send_notification");

// チェーン時にUserをNotificationに変換user_created.chain_with(&send_notification, |user: Arc<User>| {
    Notification {
        user_id: user.id,
        message: format!("ようこそ、{}さん！", user.name),
        priority: Priority::High,
    }
});
```

## 複数のシグナルのマージ

```rustlet user_login = Signal::<User>::new("user_login");
let user_signup = Signal::<User>::new("user_signup");let password_reset = Signal::<User>::new("password_reset");

// 複数のシグナルを1つにマージlet any_user_activity = Signal::merge(vec![&user_login, &user_signup, &password_reset]);

// このレシーバーは3つのイベントのいずれかでトリガーされますany_user_activity.connect(|user| async move {
    update_last_activity(&user).await?;
    Ok(())
});
```

## シグナルエミッションのフィルタリング

```rustlet user_signal = Signal::<User>::new("user_changes");

// 管理者ユーザーのみをトリガーするフィルタリングされたシグナルを作成let admin_signal = user_signal.filter(|user| user.is_admin);

admin_signal.connect(|admin_user| async move {
    log_admin_action(&admin_user).await?;
    Ok(())
});

// 管理者ユーザーのみがフィルタリングされたシグナルをトリガーしますuser_signal.send(regular_user).await?; // admin_signalをトリガーしません
user_signal.send(admin_user).await?;   // admin_signalをトリガーします
```

## シグナル値のマッピング

```rustlet user_signal = Signal::<User>::new("user_signal");

// UserをユーザーIDにマップlet user_id_signal: Signal<i32> = user_signal.map(|user: Arc<User>| user.id);

user_id_signal.connect(|user_id| async move {
    println!("ユーザーID: {}", user_id);
    Ok(())
});
```

## 複雑な合成

複数の合成演算子を組み合わせて、洗練されたイベントフローを実現：

```rustlet user_signal = Signal::<User>::new("users");

// 管理者ユーザーをフィルタリングし、そのIDにマップlet admin_ids: Signal<i32> = user_signal
    .filter(|user| user.is_admin)
    .map(|user: Arc<User>| user.id);

admin_ids.connect(|admin_id| async move {
    audit_log.record_admin_activity(*admin_id).await?;
    Ok(())
});
```

## パフォーマンスメトリクス

組み込みのメトリクス収集でシグナルのパフォーマンスを監視：

```rustlet signal = Signal::<User>::new("user_updates");

signal.connect(|user| async move {
    process_user(&user).await?;
    Ok(())
});

// シグナルを送信for i in 0..100 {
    signal.send(create_user(i)).await?;
}

// メトリクスを取得let metrics = signal.metrics();
println!("送信回数: {}", metrics.send_count);println!("レシーバー実行回数: {}", metrics.receiver_executions);
println!("成功率: {:.2}%", metrics.success_rate());println!("平均実行時間: {:?}", metrics.avg_execution_time());
println!("最小実行時間: {:?}", metrics.min_execution_time());println!("最大実行時間: {:?}", metrics.max_execution_time());

// メトリクスをリセットsignal.reset_metrics();
```

**利用可能なメトリクス:**

- `send_count` - シグナルが送信された回数
- `receiver_executions` - レシーバー実行回数の合計
- `failed_executions` - 失敗したレシーバー実行回数
- `success_rate()` - 成功率（パーセンテージ、0-100）
- `avg_execution_time()` - レシーバーの平均実行時間
- `min_execution_time()` - レシーバーの最小実行時間
- `max_execution_time()` - レシーバーの最大実行時間

**特徴:**

- アクセスしない限りゼロコスト
- スレッドセーフなアトミック操作
- クローンされたシグナル間で共有
- テストと監視のためにリセット可能

## Django vs Reinhardt Signals 比較

| 機能           | Django | Reinhardt | 備考                            |
|----------------|--------|-----------|---------------------------------|
| 送信者フィルタリング  | ✅      | ✅         | RustはTypeIdで型安全なフィルタリングを実現 |
| dispatch_uid   | ✅      | ✅         | 重複登録を防止                   |
| send_robust    | ✅      | ✅         | レシーバーが失敗しても実行を継続          |
| 弱参照         | ✅      | ✅         | SyncSignalモジュールで利用可能        |
| @receiverデコレータ | ✅      | ✅         | `connect_receiver!`マクロを使用     |
| 非同期サポート     | ⚠️     | ✅         | ネイティブasync/awaitサポート            |
| 型安全性       | ❌      | ✅         | コンパイル時型チェック                   |
| メモリ安全性      | ⚠️     | ✅         | Rust所有権システムによる保証           |

## パフォーマンス

Reinhardtシグナルはパフォーマンスを重視して設計されています：

- **Arcベースストレージ**: 効率的なレシーバーのクローン
- **並行性のためのRwLock**: 複数リーダー、単一ライター
- **ゼロアロケーション**: 送信者フィルタリング（TypeId比較）
- **非同期ランタイム**: 効率的な非同期実行のためのTokio活用

## Djangoからの移行

```python
# Django
from django.db.models.signals import post_savefrom django.dispatch import receiver

@receiver(post_save, sender=User)def on_user_saved(sender, instance, created, **kwargs):
    print(f"User saved: {instance}")
```

```rust
// Reinhardtuse reinhardt_signals::{connect_receiver, post_save};

connect_receiver!(
    post_save::<User>(),
    |instance| async move {
        println!("ユーザー保存: {:?}", instance);
        Ok(())
    },
    sender = UserModel
);
```