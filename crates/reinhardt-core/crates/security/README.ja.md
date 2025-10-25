# reinhardt-security

Reinhardtフレームワークのためのセキュリティユーティリティ。

## 概要

Webアプリケーションを保護するためのセキュリティユーティリティとミドルウェアです。CSRF保護、XSS防止、セキュリティヘッダー管理、HSTSサポートを含む包括的なセキュリティ機能を提供します。

## 機能

### 実装済み ✓

#### CSRF保護

- **トークン生成と検証**: マスキング/アンマスキングメカニズムを備えた暗号学的に安全なCSRFトークン生成
- **トークン管理**: ライフサイクル管理のための`get_secret()`、`get_token()`、`rotate_token()`関数
- **フォーマット検証**: トークンの長さと文字セットを検証する`check_token_format()`
- **トークンマッチング**: トークンを安全に比較するための`does_token_match()`
- **Origin/Refererチェック**: リクエストソースを検証する`check_origin()`と`check_referer()`
- **ドメイン検証**: クロスドメインリクエスト保護のための`is_same_domain()`
- **設定可能なCookie設定**: SameSite、Secure、HttpOnly、Domain、Path、Max-Ageの完全な制御
- **本番環境対応設定**: セキュリティ強化された`CsrfConfig::production()`
- **ミドルウェア**: カスタマイズ可能な設定を備えた`CsrfMiddleware`
- **エラー処理**: デバッグ用の詳細な拒否理由(不正なorigin、不正なreferer、トークン欠落など)

#### XSS防止

- **HTMLエスケープ**: 危険な文字(`<`、`>`、`&`、`"`、`'`)をエスケープする`escape_html()`
- **HTMLサニタイゼーション**: 基本的なHTML入力サニタイゼーションのための`sanitize_html()`
- **安全な出力**: ユーザー生成コンテンツでのスクリプトインジェクションを防止

#### セキュリティヘッダー

- **Content Security Policy (CSP)**: 以下を細かく制御できる設定可能なCSP:
  - `default-src`、`script-src`、`style-src`、`img-src`
  - `connect-src`、`font-src`、`object-src`、`media-src`、`frame-src`
- **セキュリティヘッダーミドルウェア**: 包括的なデフォルトを備えた`SecurityHeadersMiddleware`
- **設定可能なヘッダー**:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`(クリックジャッキング保護)
  - `X-XSS-Protection: 1; mode=block`
  - `Strict-Transport-Security` (HSTS)
  - `Referrer-Policy: strict-origin-when-cross-origin`
  - `Permissions-Policy` (オプション)

#### HSTS (HTTP Strict Transport Security)

- **HSTS設定**: ビルダーパターンを備えた`HstsConfig`
- **設定可能なオプション**:
  - `max_age`: 秒単位で設定可能な期間
  - `includeSubDomains`: オプションのサブドメイン保護
  - `preload`: HSTSプリロードリストサポート
- **ヘッダー生成**: 自動ヘッダー値構築のための`build_header()`
- **安全なデフォルト**: 1年間のmax-ageデフォルト設定

#### セキュリティユーティリティ

- **安全なトークン生成**: 暗号学的にランダムなトークンを作成する`generate_token()`
- **SHA-256ハッシング**: 安全な文字列ハッシングのための`hash_sha256()`
- **乱数生成**: セキュリティのための`rand`クレート上に構築

#### エラー処理

- **包括的なエラータイプ**: 特定のバリアントを持つ`SecurityError`列挙型
- **CSRF検証エラー**: デバッグ用の詳細なエラーメッセージ
- **XSS検出**: 潜在的なXSS試行のためのエラータイプ
- **設定エラー**: セキュリティ設定の検証

### 予定

現在、すべての予定された機能が実装されています。
