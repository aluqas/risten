# Sakuramiya (Risten) Roadmap

**Sakuramiya Project** の技術的ロードマップ。
Rust の型システムを極限まで活用した、ゼロコスト・ハイパフォーマンスなイベント駆動フレームワークを目指します。

> [!NOTE]
> 実装済みの機能詳細は [IMPLEMENTED.md](./IMPLEMENTED.md) を参照してください。

---

## 🏆 Current Focus: Phase 3 (Developer Experience & Architecture)

コアアーキテクチャの最適化が完了し、現在は「使いやすさ」と「基盤の洗練」に焦点を当てています。

### 3.1 宣言的マクロ (Declarative Macros)

ハンドラ定義を簡潔にするための Procedural Macros。

- [x] `#[risten::event]`: 関数を Handler/Hook に自動変換
  - 引数の型から自動で `KeyExtractor` や `FromEvent` を生成
- [x] `#[risten::main]`: ボイラープレートを排除したメイン関数定義
- [x] `#[risten::dispatch]`: EnumDispatch の自動生成 (Proc-Macro版)

### 3.2 高度なエラーハンドリング

型安全かつ柔軟なエラー処理基盤。

- [x] `IntoResponse` パターン (Axum like): ハンドラの戻り値を Result から HookResult へ自動変換
- [ ] `ErrorLayer`: エラーを一元管理・通知するミドルウェア

### 3.3 Pipeline & Flow Control (パイプライン制御)

イベント処理フローの柔軟な制御。

- [ ] **`PipelineBuilder`**: 型安全な Fluent API による動的パイプライン構築
- [ ] **`ForkJoin<H1, H2>`**: 並列実行後の結果合流
- [ ] **`Race<H1, H2>`**: 最初に完了した Hook の結果を採用
- [ ] **`Retry<H, N>`**: 失敗時の自動リトライ (Exponential Backoff)
- [ ] **Back-pressure**: 負荷制御・Flow Control
- [ ] **`Debounce<H>` / `Throttle<H>`**: イベント流量制御

### 3.4 Context & State Management (コンテキスト管理)

リクエストスコープとグローバルステートの型安全な管理。

- [ ] **`Context<T>`**: リクエストスコープの型安全コンテキスト
- [ ] **`State<T>`**: グローバル共有ステート (Read-heavy 最適化)
- [ ] **`Extension<T>`**: 動的型付けエクステンション (Tower 互換)
- [ ] **`Scope<T>`**: ライフタイム付きリソーススコープ
- [ ] **`LocalKey<T>`**: Task-local ストレージ

### 3.5 Observability & Diagnostics (可観測性)

運用・デバッグを支える計測・診断機能。

- [x] **`Trace<H>`**: `tracing` Span の自動生成・伝播 (`Trace` wrapper & `Traceable` trait)
- [x] `MetricsCollector`: Prometheus / OpenTelemetry 対応 (Trait定義のみ)
- [x] `DeadLetter`: 処理失敗イベントのキャプチャ・再試行キュー (Trait定義のみ)
- [ ] **`DebugOverlay`**: 開発時のパイプライン可視化
- [ ] **`HealthCheck`**: ヘルスチェックエンドポイント

### 3.6 Data Control & Memory Management (データ制御・メモリ管理)

メモリ効率とデータライフサイクルの精密な制御。

#### Allocation Strategy

- [ ] **`Arena<T>`**: スコープ限定バンプアロケータ (`bumpalo` 統合)
- [ ] **`Pool<T>`**: オブジェクトプール (再利用によるアロケ削減)
- [ ] **`Slab<T>`**: 固定サイズスラブアロケータ
- [ ] **`#[risten::no_alloc]`**: アロケーションフリー処理の強制

#### Data Lifecycle

- [ ] **`Owned<T>` / `Borrowed<'a, T>`**: 所有権の明示的な表現
- [ ] **`Cow<'a, T>`**: Clone-on-Write パターンの統一的サポート
- [ ] **`LazyCell<T>`**: 遅延初期化セル
- [ ] **`DropGuard<T>`**: スコープ終了時の確実なクリーンアップ

#### Buffer & Serialization

- [ ] **`BytesMut` 統合**: `bytes` クレートとの相互運用
- [ ] **`ZeroCopyBuffer`**: パース不要のワイヤーフォーマット直接アクセス
- [ ] **`PreAllocated<N>`**: コンパイル時サイズ確定バッファ
- [ ] **`StreamingParser`**: ストリーミング JSON/MessagePack パーサー

---

## 🚀 Future Vision: Phase 4 & Beyond

より挑戦的で、先進的な機能群。

---

### 4.1 Type-Level Programming (型レベルプログラミング)

Rust の型システムを「計算機」として活用し、実行時コストをゼロに。
外部の型レベルプログラミングクレートとの統合を視野に入れ、コンパイル時にあらゆる決定を行う。

#### HTree (Heterogeneous Tree)

HList (線形リスト) を拡張し、型レベルで木構造を表現。
条件分岐やルーティングをコンパイル時に静的化。

- [ ] **`Branch<Cond, Then, Else>`**: 型レベル条件分岐
- [ ] **`Match<E, Cases...>`**: 型レベルパターンマッチ (Enum Variant → Handler)
- [ ] **`TypeRouter`**: 型情報のみでルーティングを決定 (文字列比較不要)
- [ ] **静的ルーティング木の自動生成**: Proc-Macro による最適化された分岐コード生成

#### Type-Level Computation

- [ ] **`TypeMap<K, V>`**: 型をキーとする連想配列 (Frunk `HList` 拡張)
- [ ] **`TypeSet`**: 型の集合演算 (Union, Intersection, Difference)
- [ ] **`Requires<R>` / `Provides<P>`**: 依存関係の型レベル表明と検証
- [ ] **`Infer<T>`**: 型推論を活用した自動抽出 (Extractor の自動導出)

#### Compile-Time Routing

- [ ] **`const_trie!`**: コンパイル時 Trie 構築 (文字列→分岐コード)
- [ ] **`phf_router!`**: PHF を活用した O(1) ルーティングの Proc-Macro 版
- [ ] **Static Dispatch Table**: VTable を完全に排除したジャンプテーブル生成

---

### 4.2 Extreme Static Optimization (極限の静的最適化)

実行時のあらゆるオーバーヘッドを排除し、理論上の限界に挑む。

#### Zero-Copy Architecture (Advanced)

- [ ] **`LendingListener<'a>`**: 真の Lending Iterator パターン (GAT 完全活用)
- [ ] **`ParseOnDemand<T>`**: 必要なフィールドのみ遅延パース
- [ ] **`View<'a, T>`**: データ構造への借用ビュー (Clone 不要)
- [ ] **`MappedRef<'a, T, U>`**: 参照のマッピング (所有権変換なし)

#### Inlining & Monomorphization

- [ ] **`#[risten::inline_always]`**: 強制インライン化ヒント
- [ ] **Monomorphic Dispatch**: ジェネリクスの特殊化による VTable 排除
- [ ] **`StaticChain<H1, H2, ...>`**: 完全にインライン化された Hook チェーン
- [ ] **Link-Time Optimization (LTO) 最適化ガイド**: ドキュメント・ベストプラクティス

#### Memory Layout Optimization

- [ ] **`#[repr(C)]` 互換メッセージ**: FFI 対応・キャッシュ効率最大化
- [ ] **Small String Optimization (SSO)**: 短い文字列のスタック配置
- [ ] **Bitpacking**: フラグ類のビットレベル圧縮
- [ ] **Cache-Line Alignment**: `#[repr(align(64))]` による効率的メモリ配置

---

### 4.3 Distributed & Clustering (分散対応)

単一プロセスを超えたスケーラビリティ。

- [ ] **Cluster Mode**: 複数ノードでの協調動作
- [ ] **Remote Routing**: Redis / NATS / Kafka を介したイベント配送
- [ ] **State Synchronization**: CRDT 等を用いたステート共有
- [ ] **Sharding**: イベントキーによる自動シャーディング

---

### 4.4 Advanced Patterns (高度なパターン)

複雑なワークフローを扱うための抽象化。

- [ ] **Saga Pattern**: 分散トランザクション・補償トランザクション
- [ ] **Actor Model Integration**: Actix / Tokio Actor との相互運用
- [ ] **Reactive Streams**: `Stream` / `Sink` トレイトとの深い統合
- [ ] **Event Sourcing**: イベント履歴からの状態再構築
- [ ] **CQRS**: コマンドとクエリの責務分離

---

### 4.5 Experimental / Hardcore (実験的・低レイヤ)

Rust の性能を搾り尽くすための実験的機能。

- [ ] **WASM Plugin System**: WebAssembly による動的プラグインロード (Hot Reload)
- [ ] **Kernel Bypass / io_uring**: OS のオーバーヘッドを回避した超高速 I/O (Linux)
- [ ] **SIMD Parsing**: JSON/MessagePack の SIMD 最適化パース
- [ ] **Lock-Free Structures**: `crossbeam` 等を活用したロックフリーデータ構造
- [ ] **Custom Allocator**: Arena / Slab アロケータの統合

---

### 4.6 Compiler & Tooling Integration (コンパイラ連携)

開発体験と診断能力の向上。

- [ ] **Custom Lint (`rustc` Plugin)**: risten 固有の設計違反検出
- [ ] **IDE Integration**: rust-analyzer 向け補完・ホバー情報拡張
- [ ] **Build-time Report**: コンパイル時のルーティングテーブル・チェーン構造出力
- [ ] **`cargo risten`**: プロジェクトスキャフォールディング・診断ツール
