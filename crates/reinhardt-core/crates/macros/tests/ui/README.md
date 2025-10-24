# UI Tests / UIテスト

This directory contains compile-time tests for procedural macros using [`trybuild`](https://docs.rs/trybuild/).

このディレクトリには、[`trybuild`](https://docs.rs/trybuild/)を使用した手続きマクロのコンパイル時テストが含まれています。

## Directory Structure / ディレクトリ構造

```
tests/ui/
├── pass/                    # ✅ Tests that should compile successfully
│   └── *.rs                 # ✅ 正常にコンパイルされるべきテスト
├── fail/                    # ❌ Tests that should fail to compile
│   ├── *.rs                 # ❌ コンパイルエラーになるべきテスト
│   └── *.stderr             # 📄 Expected compiler error messages
│                            # 📄 期待されるコンパイラエラーメッセージ
├── path/pass/               # Path macro: success cases
├── path/fail/               # Path macro: failure cases
├── permissions/pass/        # Permission macro: success cases
├── permissions/fail/        # Permission macro: failure cases
├── routes/pass/             # Routes macro: success cases
└── routes/fail/             # Routes macro: failure cases
```

## What are `.stderr` files? / `.stderr`ファイルとは？

### English

`.stderr` files contain **expected compiler error messages** for tests in the `fail/` directories. When you run `cargo test`, `trybuild`:

1. Compiles the `.rs` test file
2. Captures the compiler error output
3. Compares it with the corresponding `.stderr` file
4. Fails the test if the error message doesn't match

**Why this extension?**
The `.stderr` extension is required by `trybuild` and follows Rust ecosystem conventions for "standard error output". This cannot be changed to a custom extension.

**Purpose:**

- Ensures macros produce clear, helpful error messages
- Prevents regressions in error message quality
- Documents expected failure modes

### 日本語

`.stderr`ファイルには、`fail/`ディレクトリ内のテスト用の**期待されるコンパイラエラーメッセージ**が保存されています。`cargo test`を実行すると、`trybuild`は以下を行います:

1. `.rs`テストファイルをコンパイル
2. コンパイラのエラー出力をキャプチャ
3. 対応する`.stderr`ファイルと比較
4. エラーメッセージが一致しない場合、テストを失敗させる

**なぜこの拡張子？**
`.stderr`拡張子は`trybuild`が要求するもので、Rustエコシステムの「標準エラー出力」の慣習に従っています。カスタム拡張子への変更はできません。

**目的:**

- マクロが明確で役立つエラーメッセージを生成することを保証
- エラーメッセージ品質の後退を防止
- 期待される失敗モードをドキュメント化

## Example / 例

For a test file `missing_path.rs`:

```rust
// This should fail because path is missing
installed_apps! {
    auth:,  // ❌ Missing path value
}
```

The corresponding `missing_path.stderr` contains:

```
error: expected string literal
 --> tests/ui/fail/missing_path.rs:8:14
  |
8 |         auth:,
  |              ^
```

This ensures the macro produces a helpful error message pointing to the exact problem location.

---

これにより、マクロが問題の正確な位置を指す有用なエラーメッセージを生成することが保証されます。

## Adding New Tests / 新しいテストの追加

1. Create a new `.rs` file in the appropriate directory
   - 適切なディレクトリに新しい`.rs`ファイルを作成
2. For `fail/` tests, run `cargo test` to generate the `.stderr` file automatically
   - `fail/`テストの場合、`cargo test`を実行して`.stderr`ファイルを自動生成
3. Review and commit both files
   - 両方のファイルをレビューしてコミット

## Reference / 参考資料

- [trybuild documentation](https://docs.rs/trybuild/)
- [compile_tests.rs](../compile_tests.rs) - Test runner implementation