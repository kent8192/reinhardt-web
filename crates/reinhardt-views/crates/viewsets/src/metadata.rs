use async_trait::async_trait;
use hyper::Method;
use reinhardt_apps::{Request, Response, Result};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// カスタムアクションのハンドラートレイト
#[async_trait]
pub trait ActionHandler: Send + Sync {
    async fn handle(&self, request: Request) -> Result<Response>;
}

/// 関数ポインタベースのActionHandler実装
pub struct FunctionActionHandler {
    handler: Arc<
        dyn Fn(Request) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> + Send + Sync,
    >,
}

impl FunctionActionHandler {
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(Request) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            handler: Arc::new(handler),
        }
    }
}

#[async_trait]
impl ActionHandler for FunctionActionHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        (self.handler)(request).await
    }
}

/// アクションのメタデータ
pub struct ActionMetadata {
    /// 関数名（デフォルトの識別子）
    pub name: String,

    /// 詳細アクション（単一オブジェクト）かリストアクションか
    pub detail: bool,

    /// カスタム表示名
    pub custom_name: Option<String>,

    /// カスタムサフィックス
    pub suffix: Option<String>,

    /// カスタムURLパス
    pub url_path: Option<String>,

    /// カスタムURL名（リバースルーティング用）
    pub url_name: Option<String>,

    /// 許可するHTTPメソッド
    pub methods: Vec<Method>,

    /// 実際のハンドラー関数
    pub handler: Arc<dyn ActionHandler>,
}

impl ActionMetadata {
    /// 新しいActionMetadataを作成
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            detail: false,
            custom_name: None,
            suffix: None,
            url_path: None,
            url_name: None,
            methods: vec![Method::GET],
            handler: Arc::new(FunctionActionHandler::new(|_| {
                Box::pin(async { Response::ok().with_json(&serde_json::json!({})) })
            })),
        }
    }

    /// 詳細アクションとして設定
    pub fn with_detail(mut self, detail: bool) -> Self {
        self.detail = detail;
        self
    }

    /// カスタム名を設定
    pub fn with_custom_name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    /// サフィックスを設定
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// URLパスを設定
    pub fn with_url_path(mut self, path: impl Into<String>) -> Self {
        self.url_path = Some(path.into());
        self
    }

    /// URL名を設定
    pub fn with_url_name(mut self, name: impl Into<String>) -> Self {
        self.url_name = Some(name.into());
        self
    }

    /// HTTPメソッドを設定
    pub fn with_methods(mut self, methods: Vec<Method>) -> Self {
        self.methods = methods;
        self
    }

    /// ハンドラーを設定
    pub fn with_handler<H: ActionHandler + 'static>(mut self, handler: H) -> Self {
        self.handler = Arc::new(handler);
        self
    }

    /// 表示名を取得（custom_name > name + suffix > name）
    pub fn display_name(&self) -> String {
        if let Some(ref custom_name) = self.custom_name {
            custom_name.clone()
        } else if let Some(ref suffix) = self.suffix {
            format!("{} {}", self.format_name(&self.name), suffix)
        } else {
            self.format_name(&self.name)
        }
    }

    /// URL名を取得（url_name > name）
    pub fn get_url_name(&self) -> String {
        self.url_name
            .clone()
            .unwrap_or_else(|| self.name.replace('_', "-"))
    }

    /// URLパスを取得（url_path > デフォルト生成）
    pub fn get_url_path(&self) -> String {
        self.url_path
            .clone()
            .unwrap_or_else(|| self.name.replace('_', "-"))
    }

    /// snake_case を Title Case に変換
    fn format_name(&self, name: &str) -> String {
        name.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Clone for ActionMetadata {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            detail: self.detail,
            custom_name: self.custom_name.clone(),
            suffix: self.suffix.clone(),
            url_path: self.url_path.clone(),
            url_name: self.url_name.clone(),
            methods: self.methods.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl fmt::Debug for ActionMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActionMetadata")
            .field("name", &self.name)
            .field("detail", &self.detail)
            .field("custom_name", &self.custom_name)
            .field("suffix", &self.suffix)
            .field("url_path", &self.url_path)
            .field("url_name", &self.url_name)
            .field("methods", &self.methods)
            .finish()
    }
}

/// アクションレジストリエントリ（inventoryで収集）
pub struct ActionRegistryEntry {
    pub viewset_type: &'static str,
    pub action_name: &'static str,
    pub metadata_fn: fn() -> ActionMetadata,
}

impl ActionRegistryEntry {
    pub const fn new(
        viewset_type: &'static str,
        action_name: &'static str,
        metadata_fn: fn() -> ActionMetadata,
    ) -> Self {
        Self {
            viewset_type,
            action_name,
            metadata_fn,
        }
    }
}

inventory::collect!(ActionRegistryEntry);

/// ViewSet型に関連するアクションを取得
pub fn get_actions_for_viewset(viewset_type: &str) -> Vec<ActionMetadata> {
    inventory::iter::<ActionRegistryEntry>()
        .filter(|entry| entry.viewset_type == viewset_type)
        .map(|entry| (entry.metadata_fn)())
        .collect()
}
