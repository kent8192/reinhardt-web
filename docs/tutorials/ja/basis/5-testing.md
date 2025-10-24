# パート5: テスト

このチュートリアルでは、アプリケーションが正しく動作することを確認するための自動テストを作成します。

## テストが重要な理由

テストは以下のことに役立ちます：

- **時間の節約**: 自動テストは手動テストよりも速くバグを検出
- **バグの防止**: テストは本番環境に入る前に予期しない動作を明らかにする
- **信頼性の構築**: よくテストされたコードは変更や拡張が容易
- **コラボレーションの促進**: テストはチームメイトによる偶発的な破損から保護

## 最初のテストを作成

`was_published_recently()`メソッドのバグを特定しましょう。`pub_date`が未来の質問に対して`True`を返しますが、これは正しくありません。

`src/models.rs`にテストを作成します：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_was_published_recently_with_future_question() {
        // 30日後の質問を作成
        let future_date = Utc::now() + Duration::days(30);
        let question = Question {
            id: 1,
            question_text: "Future question".to_string(),
            pub_date: future_date,
        };

        // 未来の質問に対してはfalseを返すべき
        assert_eq!(question.was_published_recently(), false);
    }

    #[test]
    fn test_was_published_recently_with_old_question() {
        // 2日前の質問を作成
        let old_date = Utc::now() - Duration::days(2);
        let question = Question {
            id: 1,
            question_text: "Old question".to_string(),
            pub_date: old_date,
        };

        // 1日より古い質問に対してはfalseを返すべき
        assert_eq!(question.was_published_recently(), false);
    }

    #[test]
    fn test_was_published_recently_with_recent_question() {
        // 23時間前の質問を作成
        let recent_date = Utc::now() - Duration::hours(23);
        let question = Question {
            id: 1,
            question_text: "Recent question".to_string(),
            pub_date: recent_date,
        };

        // 最近の質問に対してはtrueを返すべき
        assert_eq!(question.was_published_recently(), true);
    }
}
```

テストを実行します：

```bash
cargo test
```

最初のテストが失敗することがわかります。`src/models.rs`の`was_published_recently()`メソッドを更新してバグを修正しましょう：

```rust
impl Question {
    /// この質問が最近公開されたかチェック（過去1日以内）
    pub fn was_published_recently(&self) -> bool {
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::days(1);
        // 修正: pub_dateが未来でないこともチェック
        self.pub_date >= one_day_ago && self.pub_date <= now
    }
}
```

テストを再度実行します：

```bash
cargo test
```

すべてのテストが成功するはずです！

## ビューのテスト

Reinhardtテストクライアントを使用してビューをテストしましょう。

`Cargo.toml`にテスト依存関係を追加します：

```toml
[dev-dependencies]
reinhardt = { version = "0.1.0", features = ["test"] }
```

`src/polls.rs`にindexビューのテストを作成します：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt::prelude::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_no_questions() {
        let client = APIClient::new();
        let response = client.get("/polls/").await.unwrap();

        assert_eq!(response.status(), 200);
        assert!(response.text().contains("No polls are available"));
    }

    #[tokio::test]
    async fn test_past_question() {
        // 過去の質問を作成
        let pool = setup_test_db().await;
        let past_date = Utc::now() - chrono::Duration::days(30);

        crate::models::Question::create(
            &pool,
            "Past question.".to_string(),
            past_date,
        )
        .await
        .unwrap();

        let client = APIClient::with_pool(pool);
        let response = client.get("/polls/").await.unwrap();

        assert_eq!(response.status(), 200);
        assert!(response.text().contains("Past question"));
    }

    #[tokio::test]
    async fn test_future_question() {
        // 未来の質問を作成
        let pool = setup_test_db().await;
        let future_date = Utc::now() + chrono::Duration::days(30);

        crate::models::Question::create(
            &pool,
            "Future question.".to_string(),
            future_date,
        )
        .await
        .unwrap();

        let client = APIClient::with_pool(pool);
        let response = client.get("/polls/").await.unwrap();

        assert_eq!(response.status(), 200);
        // 未来の質問は表示されないべき
        assert!(!response.text().contains("Future question"));
    }

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }
}
```

## 詳細ビューのテスト

detailビューのテストを追加します：

```rust
#[tokio::test]
async fn test_future_question_detail() {
    let pool = setup_test_db().await;
    let future_date = Utc::now() + chrono::Duration::days(5);

    let question_id = crate::models::Question::create(
        &pool,
        "Future question.".to_string(),
        future_date,
    )
    .await
    .unwrap();

    let client = APIClient::with_pool(pool);
    let response = client.get(&format!("/polls/{}/", question_id)).await.unwrap();

    // 未来の質問に対しては404を返すべき
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_past_question_detail() {
    let pool = setup_test_db().await;
    let past_date = Utc::now() - chrono::Duration::days(5);

    let question_id = crate::models::Question::create(
        &pool,
        "Past Question.".to_string(),
        past_date,
    )
    .await
    .unwrap();

    let client = APIClient::with_pool(pool);
    let response = client.get(&format!("/polls/{}/", question_id)).await.unwrap();

    assert_eq!(response.status(), 200);
    assert!(response.text().contains("Past Question"));
}
```

## テストのベストプラクティス

1. **一度に一つのことをテスト**: 各テストは単一の動作に焦点を当てるべき
2. **説明的な名前を使用**: テスト名は何をテストするかを明確に記述すべき
3. **Arrange-Act-Assert**: セットアップ、実行、検証の構造でテストを構成
4. **テストフィクスチャを使用**: 共通のテストデータセットアップを共有
5. **エッジケースをテスト**: ハッピーパスだけでなくエッジケースもテスト

## テストの構成

テストを以下のように整理します：

- **単体テスト**: 個々の関数とメソッドをテスト（同じファイル内）
- **統合テスト**: 複数のコンポーネントを一緒にテスト（`tests/`ディレクトリ内）
- **モデルテスト**: データベースモデルとクエリをテスト
- **ビューテスト**: HTTPエンドポイントとレスポンスをテスト

## 特定のテストの実行

すべてのテストを実行：

```bash
cargo test
```

パターンに一致するテストを実行：

```bash
cargo test test_was_published
```

特定のモジュールのテストを実行：

```bash
cargo test models::tests
```

## テストカバレッジ

テストカバレッジを確認するには、`cargo-tarpaulin`を使用します：

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

これにより、どのコード行がテストされているかを示すカバレッジレポートが生成されます。

## まとめ

このチュートリアルで学んだこと：

- 自動テストが重要な理由
- モデルの単体テストの作成方法
- ビューの統合テストの作成方法
- HTTPリクエストをシミュレートするためのテストクライアントの使用方法
- テストのベストプラクティスと構成
- テストの実行と構成方法

テストはプロフェッショナルなソフトウェア開発の重要な部分です。よくテストされたコードは、自信を持ってメンテナンス、変更、デプロイすることが容易です。

## 次は何をする？

次のチュートリアルでは、投票アプリの見た目を良くするためにCSSスタイリングと画像を追加します。

[パート6: 静的ファイル](6-static-files.md)に進んでください。
