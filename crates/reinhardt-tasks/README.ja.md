# reinhardt-tasks

バックグラウンドタスク処理

## 概要

長時間実行されるタスクやスケジュール実行されるタスクを非同期で実行するためのバックグラウンドタスクキュー。

タスクスケジューリング、リトライ、タスク優先度、複数ワーカープロセスをサポートします。

## 機能

### 実装済み ✓

#### コアタスクシステム

- **Task Trait**: 基本的なタスクインターフェース
  - タスクID (`TaskId`): UUID ベースの一意識別子
  - タスク名とタスク優先度の管理
  - 優先度範囲: 0-9 (デフォルト: 5)
- **TaskExecutor Trait**: 非同期タスク実行インターフェース
- **TaskStatus**: タスクのライフサイクル管理
  - `Pending`: 待機中
  - `Running`: 実行中
  - `Success`: 成功
  - `Failure`: 失敗
  - `Retry`: リトライ中

#### タスクバックエンド

- **TaskBackend Trait**: タスクバックエンドの抽象化インターフェース
  - タスクのエンキュー (`enqueue`)
  - タスクのデキュー (`dequeue`)
  - タスクステータスの取得 (`get_status`)
  - タスクステータスの更新 (`update_status`)
- **DummyBackend**: テスト用ダミーバックエンド
  - 常に成功を返すシンプルな実装
- **ImmediateBackend**: 即座に実行するバックエンド
  - 同期的なタスク実行用
- **RedisBackend** (feature: `redis-backend`): Redis ベースの分散タスクキュー
  - Redis を使用したタスクメタデータの保存
  - キューベースのタスク配布
  - カスタマイズ可能なキープレフィックス
- **SqliteBackend** (feature: `database-backend`): SQLite ベースのタスク永続化
  - SQLite データベースでのタスク保存
  - 自動テーブル作成
  - FIFO ベースのタスク取得

#### タスクキュー

- **TaskQueue**: タスクキュー管理
  - 設定可能なキュー名
  - リトライ回数の設定 (デフォルト: 3回)
  - バックエンドを介したタスクのエンキュー
- **QueueConfig**: キュー設定
  - カスタマイズ可能なキュー名
  - 最大リトライ回数の設定

#### タスクスケジューリング

- **Scheduler**: タスクスケジューラー
  - タスクとスケジュールの登録
  - スケジュールに基づいたタスク実行の基盤
- **Schedule Trait**: スケジュールインターフェース
  - 次回実行時刻の計算
- **CronSchedule**: Cron式ベースのスケジュール
  - Cron式の保持と管理

#### ワーカーシステム

- **Worker**: タスクワーカー
  - 並行実行数の設定 (デフォルト: 4)
  - バックエンドからのタスク取得と実行
  - グレースフルシャットダウン
  - タスク処理ループ（ポーリングベース）
  - エラーハンドリングとステータス更新
  - ブロードキャストチャンネルによるシャットダウンシグナル
- **WorkerConfig**: ワーカー設定
  - ワーカー名の設定
  - 並行実行数のカスタマイズ
  - ポーリング間隔の設定 (デフォルト: 1秒)

#### タスクチェーン

- **TaskChain**: タスクチェーン管理
  - 複数タスクの順次実行
  - チェーンステータス管理（Pending, Running, Completed, Failed）
  - タスクの追加とチェーンの進行制御
- **TaskChainBuilder**: ビルダーパターンによるチェーン構築
  - 流暢なインターフェースでタスクを追加
  - 複数タスクの一括追加
- **ChainStatus**: チェーンのライフサイクル管理

#### 結果ハンドリング

- **TaskOutput**: タスク実行結果
  - タスクIDと結果の文字列表現
- **TaskResult**: タスク結果型
  - Result型によるエラーハンドリング
- **TaskResultMetadata**: ステータス付き結果メタデータ
  - ステータス、結果、エラー、タイムスタンプの管理
- **ResultBackend Trait**: 結果の永続化インターフェース
  - 結果の保存 (`store_result`)
  - 結果の取得 (`get_result`)
  - 結果の削除 (`delete_result`)
- **MemoryResultBackend**: インメモリ結果バックエンド
  - テスト用の結果ストレージ
  - RwLock による並行アクセス制御

#### リトライとバックオフ

- **RetryStrategy**: リトライ戦略の設定
  - エクスポネンシャルバックオフ (`exponential_backoff`)
  - 固定遅延 (`fixed_delay`)
  - リトライなし (`no_retry`)
  - 最大リトライ回数、初期遅延、最大遅延、倍率の設定
  - ジッター（Thundering Herd Problem 対策）のサポート
- **RetryState**: リトライ状態の追跡
  - リトライ試行回数の記録
  - 次回リトライまでの遅延計算
  - リトライ可否の判定
  - 状態のリセット

#### エラーハンドリング

- **TaskError**: タスク関連エラー
  - 実行失敗 (`ExecutionFailed`)
  - タスク未発見 (`TaskNotFound`)
  - キューエラー (`QueueError`)
  - シリアライゼーションエラー (`SerializationFailed`)
  - タイムアウト (`Timeout`)
  - 最大リトライ超過 (`MaxRetriesExceeded`)
- **TaskExecutionError**: バックエンド実行エラー
  - 実行失敗、タスク未発見、バックエンドエラー

### 予定

- **Redis/Database での結果永続化**: 永続的な結果バックエンドの実装
  - RedisResultBackend
  - DatabaseResultBackend
- **分散タスク実行の完成**: 複数ワーカーでのタスク分散処理
  - ワーカー間の負荷分散
  - タスクのロック機構
- **実際のタスク実行**: タスクデータのデシリアライゼーションと実行
  - タスクレジストリ
  - 動的タスクディスパッチ

## テスト

Redis バックエンドのテストは TestContainers を使用して実行されます:

```bash
cargo test --package reinhardt-tasks --features all-backends
```

テストは `#[serial(redis)]` 属性により直列実行され、Redis コンテナの競合を防ぎます。
