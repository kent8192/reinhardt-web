use reinhardt_exception::Result;

/// 共通設定トレイト
///
/// 各種 `*Config` 構造体はこのトレイトを実装することで、
/// バリデーションとマージの共通インターフェースを提供します。
pub trait Config: Clone + Send + Sync + 'static {
    /// 設定値の検証。問題があれば `Error::Validation` を返すこと。
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    /// もう一方の設定を上書きルールでマージする。
    /// デフォルト実装は後勝ち（`other` を優先）。
    fn merge(self, other: Self) -> Self {
        other
    }
}
