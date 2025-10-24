# reinhardt-throttling

レート制限とスロットリング

## 概要APIの乱用を防ぐためのレート制限システムです。匿名ユーザー用のAnonRateThrottle、認証済みユーザー用のUserRateThrottle、エンドポイントごとのレート制限用のScopedRateThrottleが含まれます。メモリと

Redisを含む複数のバックエンドストレージオプションをサポートします。

## 機能

- レート制限
- 匿名ユーザー用AnonRateThrottle
- 認証ユーザー用UserRateThrottle
- カスタムスコープ用ScopedRateThrottle
- カスタムスロットルクラス
