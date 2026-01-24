//! OAuth2/OIDC provider implementations

pub mod google;
pub mod github;
pub mod apple;
pub mod microsoft;

pub use google::GoogleProvider;
pub use github::GitHubProvider;
pub use apple::AppleProvider;
pub use microsoft::MicrosoftProvider;
