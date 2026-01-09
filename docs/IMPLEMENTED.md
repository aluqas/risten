# 実装済み機能 (Implemented Features)

Sakuramiya (Risten) フレームワークにおける実装済みコンポーネントと機能のカタログです。

## 1. Core Architecture (オーケストレーション)

イベント駆動アーキテクチャの中核となるコンポーネント群。

| コンポーネント | 説明                                                                                                   | 状態  | パス                       |
| :------------- | :----------------------------------------------------------------------------------------------------- | :---: | :------------------------- |
| **Dispatcher** | イベント配信の指揮者。`StandardDispatcher` により、Provider (宛先解決) と Delivery (配送戦略) を分離。 |   ✅   | `orchestrator/standard.rs` |
| **Pipeline**   | Hook の連鎖。イベント処理のメインフローを定義。                                                        |   ✅   | `model/pipeline.rs`        |
| **Hook**       | 非同期イベント処理の最小単位。ミドルウェアとしても機能。 `HookResult` (Next/Stop) を返す。             |   ✅   | `model/hook.rs`            |
| **Listener**   | 同期的な副作用観測者。イベントを受け取り、値を返さない/フローを制御しない軽量な購読者。                |   ✅   | `model/listener.rs`        |
| **Handler**    | 最終的な処理を行うユニット。Hook の一種として扱われることも、Router の末端として機能することもある。   |   ✅   | `model/handler.rs`         |

## 2. Routing (ルーティング)

効率的なイベント振り分けメカニズム。O(1) から O(log N)、プレフィックスマッチまで多様なバックエンドを提供。

| ルーター          | 特徴                                                                                      |  計算量  | 実装ファイル                             |
| :---------------- | :---------------------------------------------------------------------------------------- | :------: | :--------------------------------------- |
| **HashMapRouter** | 標準的な `HashMap` ベース。動的な追加が可能。                                             |   O(1)   | `source/router/backends/hashmap.rs`      |
| **PhfRouter**     | コンパイル時完全ハッシュ (PHF)。ゼロコスト、競合なし。                                    |   O(1)   | `source/router/backends/phf.rs`          |
| **ConstRouter**   | `const generics` を使用した固定サイズ配列 & 二分探索。 `const_router!` マクロで定義。     | O(log N) | `source/router/backends/const_router.rs` |
| **TrieRouter**    | 文字列プレフィックス木 (Trie)。キー長に依存するルックアップ。最長一致検索対応。           |   O(k)   | `source/router/backends/trie_router.rs`  |
| **MatchItRouter** | `matchit` クレートを使用した高速なパス/ワイルドカードマッチング。                         |    -     | `source/router/backends/matchit.rs`      |
| **RoutingHook**   | ルーターを Hook チェーンの一部として組み込むためのアダプタ。`KeyExtractor` でキーを抽出。 |    ✅     | `orchestrator/routing.rs`                |

## 3. Optimization (最適化)

パフォーマンスとメモリ効率を最大化するための機能。

### Static Dispatch & Macro

`Box<dyn Hook>` のオーバーヘッドを回避し、静的ディスパッチを実現。

- **`enum_hook!`**: 複数の Hook 型を一つの Enum にまとめ、`Hook` トレイトを委譲実装。
- **`enum_handler!`**: 複数の Handler 型を一つの Enum にまとめ、`Handler` トレイトを委譲実装。
- **`HList` (Static Dispatcher)**: ヘテロジニアスリストによるコンパイル時 Hook チェーン構築 (Experimental)。

### Zero-Copy / Borrowed Data

不要なクローンを避けるための仕組み。

- **`RawMessage<'a>`**: `'static` 制約を持たない、参照を保持できるイベントメッセージ特性。
- **`BorrowedListener<In>`**: GAT (Generic Associated Types) を活用し、入力データの参照を返すことができる Listener。ゼロコピーでのフィルタリングや変換に利用。

## 4. Integration & Developer Experience (統合・DX)

エコシステムとの連携と開発者体験の向上。

### Tower Integration (`tower_compat`)

Rust 非同期エコシステムの標準である `tower` クレートとの相互運用。

- **`HookService<H>`**: Risten Hook を `tower::Service` としてラップ。
- **`ServiceHook<S>`**: `tower::Service` を Risten Hook としてラップ。
- **`TowerLayerHook`**: `tower::Layer` (Timeout, RateLimit 等) を Risten Hook に適用。

### Extractor Pattern (`extractor`)

ハンドラ定義を宣言的に記述するための仕組み (Axum/Actix-web style)。

- **`FromEvent<E>`**: イベントから特定の型を抽出するトレイト。
- **`ExtractHandler`**: 関数引数に Extractor を取る関数を `Handler` としてアダプトするラッパー。
- **標準 Extractors**: `Option<T>`, `Result<T, E>`, Tuple (`(T1, T2, ...)`) 等。

## 5. Utilities (ユーティリティ)

標準で提供される便利な Hook/Listener 群。

- **Hooks**:
  - `LoggingHook` (`tracing` 対応)
  - `TimeoutHook` (処理時間制限)
  - `ConditionalHook` / `BranchHook` (条件分岐)
- **Listeners**:
  - `FilterListener` (述語によるフィルタリング)
  - `MapListener` (イベント変換)
  - `OptionalMapListener` (FilterMap)

## 6. Advanced Features (高度な機能)

フレームワークの利便性と信頼性を高める拡張機能。

### Macros (`risten-macros`)

- **`#[risten::event]`**: 非同期関数を `Hook` 実装構造体に自動変換。
- **`#[risten::main]`**: 非同期メイン関数のセットアップ（tokio::main ラッパー）。
- **`#[risten::dispatch]`**: EnumDispatch の簡易生成。

### Advanced Error Handling

- **`IntoResponse` (IntoHookOutcome)**: ハンドラの戻り値 (`Result`, `bool`, `()`) を `HookResult` (Next/Stop) に自動変換するトレイト。

### Observability

- **`MetricsCollector`**: イベント処理のメトリクス収集用トレイト。
- **`DeadLetterQueue`**: 処理失敗イベントの退避用トレイトと簡易 `InMemoryDlq` 実装。
- **Tracing**:
  - **`Trace<H>`**: 任意の Hook をラップし、実行時に `tracing::Span` を生成するラッパー。
  - **`Traceable`**: イベントから TraceID/SpanID を抽出し、分散トレーシングコンテキストを伝播させるトレイト。
