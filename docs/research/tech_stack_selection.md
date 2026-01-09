# Research: Technology Stack Selection

本プロジェクトで採用する（または検討中の）技術スタックとその選定理由。

## 1. Core Framework & Language

### Language: Rust
- **理由**: ハイパフォーマンス、メモリ安全性、並行処理性能。
- **方針**: `Arc`, `Mutex`, `Clone` の乱用を避け、Python 並みの低効率に陥らないよう注意する。

### Discord Library: Twilight
- **選定**: [Twilight](https://twilight.rs/)
- **理由**:
  - モジュール性が高く、必要な部品だけ使える。
  - `simd-json` などのゼロコピー最適化が意識されている。
  - Serenity に比べて低レベルな制御が可能。
- **構成**:
  - `twilight-model`: データ型
  - `twilight-http`: REST クライアント
  - `twilight-gateway`: WebSocket クライアント

### Async Runtime: Tokio
- **選定**: Tokio
- **理由**: Rust 非同期エコシステムのデファクトスタンダード。ライブラリの互換性が高い。

## 2. Data & Persistence

### Database
- **候補**:
  - **CockroachDB**: PostgreSQL 互換の分散 SQL DB。堅牢性が高い。
  - **ScyllaDB**: Cassandra 互換の高速 NoSQL。大量の書き込みに強い。
- **ORM / Driver**:
  - `sqlx`: 非同期、コンパイル時 SQL チェック。型安全性重視。

### Caching
- **Redis**: 定番の KVS。
- **In-Memory**: `dashmap` や `scc` などの並行ハッシュマップを用いたプロセス内キャッシュ。

## 3. Utilities & Ecosystem

### Error Handling
- `thiserror`: ライブラリ/内部エラー定義用。
- `anyhow` / `color-eyre`: アプリケーション層でのエラーハンドリング用。

### Logging / Observability
- `tracing`: 非同期対応の構造化ロギング。
- `opentelemetry`: オブザーバビリティ標準。

### Memory Optimization
- `mimalloc` / `jemalloc`: 高速アロケータ。
- `smallvec` / `tinyvec`: スタックアロケーション最適化。
- `bumpalo`: アリーナアロケーション（イベント処理スコープ用）。

### Others
- `bon`: Builder パターンの自動生成。
- `dotenvy`: 環境変数管理。
- `secrecy`: 秘密情報の保護。
