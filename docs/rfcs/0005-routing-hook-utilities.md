# RFC: RoutingHook と汎用 Listener/Hook ユーティリティの設計

- **Status**: Draft
- **Type**: Architecture / DX Track
- **Related**: RFC 0003 (DX), RFC 0004 (Static Optimization), ROADMAP.md Phase 1, Phase 3

---

## 1. 背景と動機

`risten` フレームワークは「全ては Hook のチェーン」という統一パラダイムを基盤としています。
`Router` は存在しますが、現状では Hook チェーンとは独立したコンポーネントであり、
パイプライン内でルーティングを行うには手動での統合が必要です。

**課題:**
1. `Router` と `Hook` チェーンの間に統合レイヤーがない
2. 一般的なフィルタリング・変換 Listener が標準提供されていない
3. ロギング、タイムアウト等の横断的関心事を表す標準 Hook がない

**目標:**
- `RoutingHook<R>`: Router を Hook としてラップし、パイプライン内でルーティングを可能にする
- 汎用 Listener ユーティリティ: `FilterListener`, `MapListener`, `PrefixListener` 等
- 汎用 Hook ユーティリティ: `LoggingHook`, `TimeoutHook`, `MetricsHook` 等

---

## 2. 設計: `RoutingHook<R, F>`

### 2.1 概念

`RoutingHook` は `Router` を内包し、イベントからルーティングキーを抽出して
対応するサブ `Dispatcher` (または `Hook`) にディスパッチする `Hook` 実装です。

```text
Event ────► RoutingHook<Router, KeyExtractor> ─┬─► "ping" ─► PingDispatcher
                                               ├─► "echo" ─► EchoDispatcher
                                               └─► "help" ─► HelpDispatcher
```

### 2.2 トレイト設計

```rust
/// イベントからルーティングキーを抽出する関数/クロージャ
pub trait KeyExtractor<E: Message>: Send + Sync + 'static {
    type Key: Send + Sync;

    fn extract(&self, event: &E) -> Option<Self::Key>;
}

// 関数ポインタ/クロージャに対する blanket impl
impl<E, K, F> KeyExtractor<E> for F
where
    E: Message,
    K: Send + Sync,
    F: Fn(&E) -> Option<K> + Send + Sync + 'static,
{
    type Key = K;

    fn extract(&self, event: &E) -> Option<K> {
        (self)(event)
    }
}
```

### 2.3 `RoutingHook` 構造体

```rust
/// Router を Hook チェーン内で使用可能にするラッパー
pub struct RoutingHook<R, F, E>
where
    R: Router<F::Key, Box<dyn DynDispatcher<E>>>,
    F: KeyExtractor<E>,
    E: Message,
{
    router: R,
    extractor: F,
    fallback: Option<Box<dyn DynDispatcher<E>>>,
    _marker: PhantomData<E>,
}

impl<R, F, E> RoutingHook<R, F, E>
where
    R: Router<F::Key, Box<dyn DynDispatcher<E>>>,
    F: KeyExtractor<E>,
    E: Message + Clone + Send + Sync,
{
    pub fn new(router: R, extractor: F) -> Self {
        Self {
            router,
            extractor,
            fallback: None,
            _marker: PhantomData,
        }
    }

    pub fn with_fallback<D: DynDispatcher<E> + 'static>(mut self, fallback: D) -> Self {
        self.fallback = Some(Box::new(fallback));
        self
    }
}

impl<R, F, E> Hook<E> for RoutingHook<R, F, E>
where
    R: Router<F::Key, Box<dyn DynDispatcher<E>>>,
    F: KeyExtractor<E>,
    E: Message + Clone + Send + Sync,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        // 1. キーを抽出
        let key = match self.extractor.extract(event) {
            Some(k) => k,
            None => return Ok(HookResult::Next), // キーなし → スキップ
        };

        // 2. ルーティング
        match self.router.route(&key) {
            RouteResult::Matched(dispatcher) => {
                // 3. 対応する Dispatcher を実行
                dispatcher.dispatch_dyn(event.clone()).await?;
                Ok(HookResult::Stop) // ルーティング後は停止 (設定可能にする?)
            }
            RouteResult::NotFound => {
                // 4. Fallback があれば実行
                if let Some(ref fallback) = self.fallback {
                    fallback.dispatch_dyn(event.clone()).await?;
                    Ok(HookResult::Stop)
                } else {
                    Ok(HookResult::Next) // ルート見つからず → 次の Hook へ
                }
            }
        }
    }
}
```

### 2.4 使用例

```rust
use risten::{RoutingHook, HashMapRouter, StaticDispatcher, static_hooks};

// 各コマンド用の Dispatcher を定義
let ping_dispatcher = StaticDispatcher::new(static_hooks![PingHandler]);
let echo_dispatcher = StaticDispatcher::new(static_hooks![EchoHandler]);

// Router を構築
let mut router = HashMapRouterBuilder::new();
router.insert("ping", Box::new(ping_dispatcher) as _);
router.insert("echo", Box::new(echo_dispatcher) as _);
let router = router.build()?;

// RoutingHook を作成
let routing_hook = RoutingHook::new(router, |event: &CommandEvent| {
    Some(event.command_name.as_str())
});

// メインパイプラインに組み込み
let main_chain = static_hooks![
    LoggingHook,
    RateLimitHook,
    routing_hook,  // ← ここでルーティング
    FallbackHandler,
];
```

---

## 3. 汎用 Listener ユーティリティ

### 3.1 `FilterListener<F>`

条件に一致するイベントのみを通過させる Listener。

```rust
/// 条件付きフィルタリング Listener
pub struct FilterListener<F> {
    predicate: F,
}

impl<F> FilterListener<F> {
    pub fn new(predicate: F) -> Self {
        Self { predicate }
    }
}

impl<E, F> Listener<E> for FilterListener<F>
where
    E: Message + Clone,
    F: Fn(&E) -> bool + Send + Sync + 'static,
{
    type Output = E;

    fn listen(&self, event: &E) -> Option<E> {
        if (self.predicate)(event) {
            Some(event.clone())
        } else {
            None
        }
    }
}

// 使用例
let guild_only = FilterListener::new(|msg: &DiscordMessage| msg.guild_id.is_some());
```

### 3.2 `MapListener<F>`

イベントを変換する Listener。

```rust
/// イベント変換 Listener
pub struct MapListener<F> {
    mapper: F,
}

impl<In, Out, F> Listener<In> for MapListener<F>
where
    In: Message,
    Out: Message,
    F: Fn(&In) -> Out + Send + Sync + 'static,
{
    type Output = Out;

    fn listen(&self, event: &In) -> Option<Out> {
        Some((self.mapper)(event))
    }
}

// 使用例
let extract_content = MapListener::new(|msg: &DiscordMessage| msg.content.clone());
```

### 3.3 `PrefixListener`

コマンドプレフィックスを解析する Listener。

```rust
/// Prefix コマンド抽出 Listener
pub struct PrefixListener {
    prefix: String,
}

#[derive(Clone)]
pub struct CommandParsed {
    pub command: String,
    pub args: Vec<String>,
    pub raw: String,
}

impl Listener<ChatMessage> for PrefixListener {
    type Output = CommandParsed;

    fn listen(&self, event: &ChatMessage) -> Option<CommandParsed> {
        if !event.content.starts_with(&self.prefix) {
            return None;
        }

        let rest = &event.content[self.prefix.len()..];
        let parts: Vec<&str> = rest.split_whitespace().collect();

        parts.first().map(|cmd| CommandParsed {
            command: cmd.to_string(),
            args: parts.into_iter().skip(1).map(String::from).collect(),
            raw: rest.to_string(),
        })
    }
}
```

### 3.4 `OptionalMapListener<F>`

条件付き変換 (filter_map) を行う Listener。

```rust
/// Option を返す変換 Listener (filter_map 相当)
pub struct OptionalMapListener<F> {
    mapper: F,
}

impl<In, Out, F> Listener<In> for OptionalMapListener<F>
where
    In: Message,
    Out: Message,
    F: Fn(&In) -> Option<Out> + Send + Sync + 'static,
{
    type Output = Out;

    fn listen(&self, event: &In) -> Option<Out> {
        (self.mapper)(event)
    }
}
```

---

## 4. 汎用 Hook ユーティリティ

### 4.1 `LoggingHook`

イベント処理をログ出力する Hook。

```rust
use tracing::{info, span, Level};

/// トレーシング対応ロギング Hook
pub struct LoggingHook {
    level: Level,
}

impl<E: Message + std::fmt::Debug> Hook<E> for LoggingHook {
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        let _span = span!(Level::INFO, "event", ?event).entered();
        info!("Processing event");
        Ok(HookResult::Next)
    }
}
```

### 4.2 `TimeoutHook<H>`

内部の Hook にタイムアウトを設定する Hook。

```rust
use tokio::time::{timeout, Duration};

/// タイムアウト付き Hook ラッパー
pub struct TimeoutHook<H> {
    inner: H,
    duration: Duration,
}

impl<E: Message + Sync, H: Hook<E>> Hook<E> for TimeoutHook<H> {
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        match timeout(self.duration, self.inner.on_event(event)).await {
            Ok(result) => result,
            Err(_) => Err("Hook execution timed out".into()),
        }
    }
}
```

### 4.3 `CatchUnwindHook<H>`

panic をキャッチしてエラーに変換する Hook。

```rust
use std::panic::AssertUnwindSafe;
use futures::FutureExt;

/// Panic 安全 Hook ラッパー
pub struct CatchUnwindHook<H> {
    inner: H,
}

impl<E: Message + Sync, H: Hook<E>> Hook<E> for CatchUnwindHook<H> {
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        AssertUnwindSafe(self.inner.on_event(event))
            .catch_unwind()
            .await
            .map_err(|_| "Hook panicked".into())?
    }
}
```

### 4.4 `MetricsHook`

処理時間等のメトリクスを記録する Hook。

```rust
use std::time::Instant;

/// メトリクス収集 Hook
pub struct MetricsHook {
    // metrics handle (prometheus, opentelemetry, etc.)
}

impl<E: Message + Sync> Hook<E> for MetricsHook {
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        let start = Instant::now();
        // ここでは記録のみ（次の Hook で実際の処理）
        // 実際には Span や Context を使って後で計測
        Ok(HookResult::Next)
    }
}
```

### 4.5 `ConditionalHook<C, H>`

条件付きで実行する Hook。

```rust
/// 条件付き Hook 実行
pub struct ConditionalHook<C, H> {
    condition: C,
    inner: H,
}

impl<E, C, H> Hook<E> for ConditionalHook<C, H>
where
    E: Message + Sync,
    C: Fn(&E) -> bool + Send + Sync + 'static,
    H: Hook<E>,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        if (self.condition)(event) {
            self.inner.on_event(event).await
        } else {
            Ok(HookResult::Next)
        }
    }
}
```

---

## 5. ディレクトリ構成提案

```text
risten/risten/src/
├── model/
│   ├── hook.rs
│   ├── listener.rs
│   ├── ...
│   └── utils/                 # [NEW] 汎用ユーティリティ
│       ├── mod.rs
│       ├── filter_listener.rs
│       ├── map_listener.rs
│       ├── prefix_listener.rs
│       └── optional_map.rs
├── orchestrator/
│   ├── ...
│   └── hooks/                 # [NEW] 標準 Hook 群
│       ├── mod.rs
│       ├── logging.rs
│       ├── timeout.rs
│       ├── catch_unwind.rs
│       ├── metrics.rs
│       ├── conditional.rs
│       └── routing.rs         # RoutingHook
└── ...
```

---

## 6. 実装優先度

| 項目                    | 難易度 | インパクト | 優先度 |
| :---------------------- | :----: | :--------: | :----: |
| `RoutingHook<R, F>`     |   ⭐⭐   |     高     |   🥇    |
| `FilterListener<F>`     |   ⭐    |     中     |   🥇    |
| `MapListener<F>`        |   ⭐    |     中     |   🥇    |
| `PrefixListener`        |   ⭐    |     中     |   🥇    |
| `LoggingHook`           |   ⭐    |     中     |   🥈    |
| `TimeoutHook<H>`        |   ⭐⭐   |     高     |   🥈    |
| `CatchUnwindHook<H>`    |   ⭐⭐   |     中     |   🥈    |
| `ConditionalHook<C, H>` |   ⭐    |     中     |   🥈    |
| `MetricsHook`           |   ⭐⭐   |     中     |   🥉    |

---

## 7. 次のステップ

1. **Phase 1**: `RoutingHook` と基本 Listener ユーティリティの実装
2. **Phase 2**: 標準 Hook ユーティリティの実装 (Logging, Timeout)
3. **Phase 3**: 統合テストとドキュメント整備
4. **Phase 4**: `#[risten::event]` マクロとの統合検討

---

_最終更新: 2026-01-05_
