# Research: Ecosystem Survey

Rust エコシステムおよびクロスランゲージのイベント処理に関する調査です。

## 1. Rust エコシステム比較

### 1.1 低レベル同期プリミティブ

| クレート         | 概要                          | risten との関係             |
| ---------------- | ----------------------------- | --------------------------- |
| `event-listener` | eventcount 風同期プリミティブ | より低レベル                |
| `async-event`    | 効率的 async eventcount       | `event-listener` の効率化版 |

### 1.2 Pub/Sub・イベントエミッター

| クレート              | 特徴                                        | risten との比較                       |
| --------------------- | ------------------------------------------- | ------------------------------------- |
| `async_pub_sub`       | Publisher/Subscriber trait、middleware 対応 | risten はより型安全なパイプライン重視 |
| `pubsub_rs`           | `DashMap` + `async-channel`、topic ベース   | トピックルーティングは Router が担当  |
| `async-event-emitter` | 複数ランタイム対応、強型付き                | risten は静的ディスパッチも提供       |
| `eventador`           | LMAX Disruptor 風、lock-free                | 高スループット向け                    |

### 1.3 イベントバス・ディスパッチャ

| クレート       | 特徴                                | risten との比較             |
| -------------- | ----------------------------------- | --------------------------- |
| `basu`         | async/sync 両対応                   | `DeliveryStrategy` で抽象化 |
| `event_bus_rs` | runtime agnostic, bounded channels  | 自動トピッククリーンアップ  |
| `mod-events`   | ゼロオーバーヘッド、priority ベース | Priority Hook 追加候補      |

### 1.4 リアクティブプログラミング

| クレート          | パラダイム             | risten との比較         |
| ----------------- | ---------------------- | ----------------------- |
| `RxRust`          | ReactiveX (Observable) | Push ベース、演算子豊富 |
| `futures-signals` | FRP (Signal)           | Pull ベース、ゼロコスト |

### 1.5 Tower / Middleware パターン

**Tower `Service` vs risten `Hook`**:

| 側面     | Tower                  | risten                   |
| -------- | ---------------------- | ------------------------ |
| 入力     | 所有権取得 (`Request`) | 参照借用 (`&E`)          |
| 出力     | Response 型            | `HookResult` (Next/Stop) |
| チェーン | `Layer` でラップ       | `Pipeline` で合成        |
| 用途     | HTTP/RPC ミドルウェア  | イベント処理パイプライン |

### 1.6 CQRS / Event Sourcing

| クレート     | 特徴                                      |
| ------------ | ----------------------------------------- |
| `cqrs-es`    | 軽量 CQRS/ES、Aggregate + Command + Event |
| `eventstore` | EventStoreDB gRPC クライアント            |
| `esrc`       | ES/CQRS プリミティブ、NATS Jetstream 対応 |

### 1.7 Actor Model & ECS

| クレート   | 特徴                                          | risten との比較                                                                   |
| ---------- | --------------------------------------------- | --------------------------------------------------------------------------------- |
| `actix`    | Actor Model、メッセージパッシング、Context    | アクターのライフサイクル管理 vs パイプライン処理。`risten` は状態管理を含まない。 |
| `bevy_ecs` | ECS イベント (`EventReader/Writer`)、Observer | フレーム単位のバッチ処理が特徴。`risten` は即時処理指向。                         |
| `tauri`    | IPC イベントシステム、Frontend/Backend        | アプリ境界を越えるイベント。`risten` はプロセス内が主戦場。                       |

## 2. クロスランゲージ比較

### 2.1 JavaScript / TypeScript

| フレームワーク   | パラダイム        | 特徴                              |
| ---------------- | ----------------- | --------------------------------- |
| **EventEmitter** | Observer          | Node.js 組み込み、`on()`/`emit()` |
| **RxJS**         | ReactiveX         | 豊富な演算子、バックプレッシャー  |
| **Discord.js**   | EventEmitter 拡張 | discord.py の JS 版               |

### 2.2 Java

| フレームワーク       | パラダイム                | 特徴                              |
| -------------------- | ------------------------- | --------------------------------- |
| **Spring Events**    | ApplicationEvent/Listener | DI 統合、`@EventListener`         |
| **Project Reactor**  | Reactive Streams          | `Mono`/`Flux`、バックプレッシャー |
| **Vert.x Event Bus** | Actor 風                  | 分散イベントバス、高性能          |

### 2.3 C# / .NET

| フレームワーク       | パラダイム | 特徴                             |
| -------------------- | ---------- | -------------------------------- |
| **delegate + event** | 組み込み   | 基本的な Observer                |
| **MediatR**          | Mediator   | `IRequest`/`INotification`、CQRS |
| **Rx.NET**           | ReactiveX  | `IObservable<T>`                 |

### 2.4 Go (Golang)

| パターン/ライブラリ       | 特徴                                                   | risten との比較                                                                                                                                                  |
| ------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Goroutines + Channels** | 言語組み込み、CSP (Communicating Sequential Processes) | Go は「共有メモリによる通信」より「通信によるメモリ共有」を推奨。`risten` は所有権と借用 (`&Message`) を活用し、ロック競合を避けつつデータを共有するアプローチ。 |
| **Watermill**             | メッセージルーター、Pub/Sub 抽象化                     | `Watermill` の `Router` は `risten` の `Dispatcher` に非常に近い概念。ミドルウェアでリトライや相関 ID を扱う設計は `risten` の拡張において大いに参考になる。     |

### 2.5 Kotlin (Coroutines)

| フレームワーク       | 特徴                          | risten との比較                                                                                                     |
| -------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **Flow (Cold)**      | 遅延評価ストリーム、Rx 簡略化 | `risten` の `Listener` は同期変換だが、`Flow` は非同期変換も可。Structured Concurrency によりキャンセル処理が安全。 |
| **SharedFlow (Hot)** | ブロードキャスト、リプレイ    | `risten` の `FanoutDelivery` に近い。バッファリング戦略 (`BufferOverflow.DROP_OLDEST` 等) は参考にすべき。          |
| **StateFlow**        | 状態保持 (`Value` プロパティ) | `risten` には「現在の状態」を保持する標準機能がない。状態管理とイベント通知の一体化は UI アプリ等で強力。           |

### 2.6 Elixir (OTP)

| コンポーネント        | 特徴                                    | risten との比較                                                                                                                                                                 |
| --------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **GenServer**         | 状態を持つプロセス、同期/非同期呼び出し | `risten` のハンドラはステートレスが基本。GenServer のような「状態 + 振る舞い」のアクターモデルを取り入れるなら、ハンドラに内部状態 (`Arc<Mutex<State>>`) を持たせる必要がある。 |
| **Supervision Trees** | プロセス監視、自動再起動                | Rust では `panic` はスレッド境界でのみ捕捉可能。`risten` で「ハンドラがクラッシュしてもパイプライン全体は死なない」耐障害性を持たせるには、`CatchUnwind` Hook 等が必要。        |
| **Phoenix.PubSub**    | 分散 Pub/Sub                            | クラスタ透過なメッセージング。`risten` が分散対応するなら、バックエンド透過な `RemoteDelivery` トレイトが必要。                                                                 |

## 3. Rust エコシステムの現状分析

### 「定番がない」問題

| 言語       | 「定番」イベントフレームワーク |
| ---------- | ------------------------------ |
| JavaScript | EventEmitter, RxJS             |
| Java       | Spring Events, Reactor         |
| C#         | delegate/event, MediatR        |
| Python     | discord.py (Discord), asyncio  |
| **Rust**   | **定番不在**                   |

### 理由

1. **言語哲学**: ゼロコスト抽象化重視で vtable/動的ディスパッチに慎重
2. **所有権/ライフタイム**: `'static` + `Send + Sync` 制約で設計が複雑化
3. **パーツ止まり**: 低レベルパーツはあるがアプリレベル抽象がない
4. **文化**: 「大きなフレームワーク」より「小さなビルディングブロック」
