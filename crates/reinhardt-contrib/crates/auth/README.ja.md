# reinhardt-auth

認証と認可 (JWT、セッション、パーミッション)

## 概要

包括的な認証・認可システムです。JWTトークン、セッションベース認証、トークン認証、Basic認証をサポートします。パーミッションクラス（IsAuthenticated、IsAdminUser、カスタムパーミッション）、ユーザーモデル、Argon2によるパスワードハッシュ化が含まれます。

## 機能
- JWT（JSON Web Token）認証
- セッションベース認証
- トークン認証
- 基本HTTP認証
- パーミッションクラス（IsAuthenticated、IsAdminUserなど）
- パスワードハッシュ化を伴うユーザーモデル（Argon2）
- グループとパーミッションシステム
- カスタム認証バックエンド

