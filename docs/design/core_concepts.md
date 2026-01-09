# Core Concepts & Unified Paradigm

## 1. コア概念 (Core Concepts)

`risten` のアーキテクチャを構成する基本的な構成要素です。

| コンポーネント       | 役割                         | シグネチャ                     |
| -------------------- | ---------------------------- | ------------------------------ |
| **Message**          | イベント型マーカー           | `Clone + Send + 'static`       |
| **Listener**         | 同期フィルタ/変換 (Phase 1)  | `&In → Option<Output>`         |
| **Handler**          | 非同期終端処理 (Phase 2)     | `In → impl Future<Output>`     |
| **Pipeline**         | Listener + Handler 結合      | `Hook` として登録可能          |
| **Hook**             | 低レベルイベント注入         | `&E → Future<HookResult>`      |
| **Dispatcher**       | イベント配送オーケストレータ | Generic over Source + Delivery |
| **DeliveryStrategy** | 配送戦略                     | Sequential, Fanout 等          |
| **Static Dispatch**  | HList ゼロコスト抽象         | コンパイル時最適化             |

### 独自性

1. **2フェーズ分離**: 同期 `Listener` と非同期 `Handler` の明確な責務分離
2. **Hybrid Dispatch**: 動的 `Registry` + 静的 `HList` の両方をサポート
3. **ゼロコスト静的チェーン**: `HookChain` によるコンパイル時最適化
4. **プラガブル Delivery**: 配送戦略の差し替えが容易

---

## 2. 統一パラダイム：「全ては Hook のチェーン」

risten の設計において最も重要な洞察は、**Router も Filter も Middleware も、本質的には「イベントを見て何かする」という同じ概念**であるということです。

### 核心の洞察

| 概念           | 入力     | 処理          | 出力                 |
| -------------- | -------- | ------------- | -------------------- |
| **Listener**   | `&Event` | フィルタ/変換 | `Option<Output>`     |
| **Router**     | `&Key`   | ルックアップ  | `RouteResult<Value>` |
| **Middleware** | `&Event` | 前後処理      | `HookResult`         |

よく見ると、**全て「イベントを見て、次にどうするか決める」**という本質は同じです。この統一的な視点が risten の設計哲学です。

### 統一モデル

```text
┌─────────────────────────────────────────────────────────────┐
│                    StaticDispatcher<Chain>                  │
│  Chain = HCons<LoggingHook,          ← ミドルウェア         │
│          HCons<RateLimitHook,        ← ミドルウェア         │
│          HCons<RoutingHook<Router>,  ← Router を Hook 化    │
│          HCons<FallbackHandler,      ← デフォルトハンドラ   │
│          HNil>>>>                                           │
└─────────────────────────────────────────────────────────────┘
```

### Listener と Hook の使い分け

| 特性             | Listener                     | Hook                           |
| ---------------- | ---------------------------- | ------------------------------ |
| **同期/非同期**  | 同期 (`fn`)                  | 非同期 (`async fn`)            |
| **主な用途**     | フィルタ、変換、抽出         | ルーティング、副作用、I/O      |
| **データフロー** | `Option<Output>` で次へ繋ぐ  | `HookResult::Next/Stop` で制御 |
| **合成**         | `Pipeline` で Handler と結合 | `HCons` でチェーン化           |

**原則:**
- **Listener**: 同期で十分な軽量処理（文字列パース、条件フィルタ）
- **Hook**: 非同期が必要な処理（I/O、サブパイプライン実行、タイムアウト）
