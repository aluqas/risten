# RFC 0003: Developer Experience Improvements

- **Status**: Proposed
- **Type**: DX Track

## Summary
`risten` の開発者体験 (DX) を `discord.py` レベルまで引き上げるための API 設計案。

## 1. 3層体験設計

### Level 3: 宣言的 API (discord.py 風)

```rust
#[risten::event]
async fn on_message(msg: &Message) {
    println!("Received: {}", msg.content);
}

#[risten::listen("on_message")]
async fn log_message(msg: &Message) {
    info!("Log: {}", msg.content);
}
```

### Level 2: trait ベース (Serenity 風)

```rust
#[risten::event_handler]
impl DiscordEvents for MyHandler {
    async fn message(&self, ctx: &Context, msg: &Message) { ... }
    async fn ready(&self, ctx: &Context, ready: &Ready) { ... }
}
```

### Level 1: プリミティブ API (risten ネイティブ)

```rust
let chain = static_hooks![LoggingHook, MetricsHook, MyPipeline];
let dispatcher = StaticDispatcher::new(chain);
```

## 2. 拡張ロードマップ

### 短期 (Phase 1-2)

| 項目                   | 参考       | 説明                              |
| ---------------------- | ---------- | --------------------------------- |
| **ビルダー API**       | -          | Fluent な `Dispatcher::builder()` |
| **`#[event]` マクロ**  | discord.py | 宣言的関数ベース                  |
| **`#[listen]` マクロ** | discord.py | 複数リスナー登録                  |

### 中期 (Phase 3-4)

| 項目                   | 参考       | 説明                 |
| ---------------------- | ---------- | -------------------- |
| **`#[event_handler]`** | Serenity   | trait ベースハンドラ |
| **`define_events!`**   | Serenity   | イベント定義マクロ   |
| **HookBehavior**       | MediatR    | ミドルウェアパターン |
| **Cog システム**       | discord.py | モジュール化         |

### 長期 (Phase 5+)

| 項目               | 参考           | 説明                    |
| ------------------ | -------------- | ----------------------- |
| **wait_for**       | discord.py     | インタラクティブ待機    |
| **ホットリロード** | discord.py Cog | 動的モジュール読み込み  |
| **分散配送**       | Watermill      | Redis/NATS バックエンド |
