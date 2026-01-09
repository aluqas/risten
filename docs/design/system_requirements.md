# System Design Requirements

本プロジェクトにおけるシステム全体の非機能要件および設計指針を定義します。

## 1. Reliability & Robustness (信頼性と堅牢性)

### メッセージ送信の堅牢化
- **権限チェック**: コマンド実行前に、Bot および実行ユーザーの権限を厳密にチェックする。
- **サニタイズ**: ユーザー入力をそのまま出力せず、メンションや特殊文字を適切にエスケープする。
- **レート制限**: Discord API の Rate Limit をハンドリングし、再試行やキューイングを行う。

### 安全性 (Safety)
- **直接アクセスの禁止**: ビジネスロジックから生の HTTP クライアントや `send()` メソッドを直接叩くことを禁止する。必ずラッパー（Action Runner）を経由させる。

### エラーハンドリング
- **エラー分類**: `UserError`（ユーザー起因）、`DependencyError`（外部要因）、`Bug`（内部エラー）を明確に区別する。
- **分離**: ユーザーに表示するメッセージと、内部ログに残す詳細情報を分離する。

## 2. Observability (可観測性)

- **Structured Logging**: JSON 形式などの構造化ログを採用し、検索・分析を容易にする。
- **Metrics**: Prometheus 等でメトリクスを収集し、Bot の健康状態を可視化する。
- **Tracing**: リクエストごとの分散トレーシングを導入し、ボトルネックを特定可能にする。

## 3. Architecture Patterns (アーキテクチャパターン)

### 入口で正規化、出口で統制
- **Input (Gateway)**: 不確実な Gateway イベントを、信頼できるドメインイベント（ID 中心）へ正規化する。
- **Output (Action Runner)**: 全ての副作用（API コール、DB 書き込み）を一箇所で統制し、安全装置（権限チェック、監査ログ）を適用する。

### Persistence (永続化)
- **Repository Pattern**: データアクセスを抽象化し、DB の変更に強くする。
- **Caching**: 頻繁な API コールを避けるため、適切なキャッシュ戦略を持つ。ただしメモリ効率を考慮する。

## 4. Operational Lifecycle (運用)

- **Feature Flags**:機能を動的にオン/オフできるフラグ管理。
- **Environment Separation**: Dev / Prod 環境の分離と設定管理。
- **Hot Reload**: 再起動なしでの設定変更や、WASM プラグインによるロジック更新（将来構想）。
