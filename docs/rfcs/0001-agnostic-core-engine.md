# risten Core Proposal: 汎用イベントエンジンとしての進化

本ドキュメントでは、`risten` を Discord に依存しない **「汎用・超高性能イベント処理エンジン」** として進化させるための機能提案を行います。
特に、ユーザーから高評価を得た **「Bump Allocation (アリーナアロケーション)」** を中心に、コア部分の最適化と拡張性を定義します。

---

## 1. メモリ管理戦略: "Phase 1" Bump Allocation

イベント駆動アーキテクチャにおいて、**「大半のイベントは無視される（興味がない）」** という特性があります。
`risten` の「Listener (Sync) / Handler (Async)」という2フェーズ分離設計は、この特性を活かすのに最適な構造をしています。ここへ Bump Allocation を導入することで、**「無視されるイベントのコストを極限までゼロにする」** ことが可能です。

### アーキテクチャ概要

1. **Ingress (入口)**: 生データ（バイト列 or JSON文字列）を受け取る。
2. **Arena Creation**: そのイベント処理スコープ専用の `Bump` (Arena) をスタック上に作成する。
3. **Zero-Copy Parsing**: JSONなどをパースする際、文字列の実体をヒープにコピーせず、元のバッファを指すか、必要な構造体を `Bump` 上に確保する。
4. **Listener Execution (Phase 1)**:
   - 全て `&'bump T` (アリーナ上の参照) としてデータを扱う。
   - フィルタリング、ルーティング、簡単な加工を行う。
   - **ここでのアロケーションは `Bump` に対して行われるため、malloc/free のオーバーヘッドはゼロ（ポインタ操作のみ）。**
5. **Decision (分岐)**:
   - **Drop (無視)**: ハンドラが見つからなかった場合、即座にリターン。`Bump` が破棄され、メモリは一括解放される。**ヒープアロケーションは発生しない。**
   - **Handle (処理)**: ハンドラを実行する場合のみ、必要なデータを `ToOwned / Clone` してヒープ（Arc/Box）に移動し、非同期ランタイムへ渡す。

### 実装イメージ (Rust)

```rust
use bumpalo::Bump;

// 汎用的なイベントコンテナ
struct EventContext<'a> {
    arena: &'a Bump,
    raw_data: &'a [u8],
    // パース済みの構造体はアリーナ上に置かれる
    parsed_header: Option<&'a Header>,
}

impl Dispatcher {
    fn dispatch(&self, raw_data: &[u8]) {
        // 1. アリーナ作成 (非常に高速)
        let arena = Bump::new();

        // 2. コンテキスト作成
        let mut ctx = EventContext {
            arena: &arena,
            raw_data,
            parsed_header: None,
        };

        // 3. Listener (Phase 1) - 同期処理
        // ここでのデータ生成はすべて &arena 上で行う
        if let Some(handler) = self.router.route(&mut ctx) {
            // 4. Boundary Crossing (Phase 2への昇格)
            // 必要なデータだけを Clone して非同期タスクへ
            let owned_data = ctx.to_owned_data();
            tokio::spawn(async move {
                handler.handle(owned_data).await;
            });
        }

        // 5. アリーナ破棄 (一括解放)
    }
}
```

### Discord 以外のユースケースでの利点

*   **HTTP サーバー (Web)**: ルーティングと認証チェックだけ行い、静的ファイルを返すようなケースでアロケーションを極小化。
*   **IoT / メッセージバス**: MQTT等の大量のセンサーデータストリームに対し、閾値を超えたものだけを処理するフィルタリングエンジンとして最強の性能を発揮。
*   **ログプロセッサ**: ログ行をパースし、「エラー」だけを抽出してDBに入れる。パースに伴う一時文字列の確保コストを無視できる。

---

## 2. 汎用ルーティングとマッチング

Discord のコマンド体系に依存しない、汎用的なルーティング機構をコアに組み込みます。

### A. Hierarchical Topic Matching (MQTT-like)
イベントの「種別」をパスとして表現し、ワイルドカードマッチングを提供します。

*   **パターン**: `sensor/+/temp`, `logs/#`
*   **バックエンド**: Trie木 (Radix Tree)
*   **用途**: Pub/Sub システム、イベントバス

### B. Pattern Matching Router (Web-like)
URLパスや特定の文字列パターンに基づくルーティング。

*   **パターン**: `/users/:id/update`, `/files/*path`
*   **ライブラリ**: `matchit` (Axum等で使用) を標準アダプタとして提供。
*   **用途**: Webフックレシーバ、APIゲートウェイ

### C. Content-Based Routing (Rule Engine)
ペイロードの中身（値）に基づいて動的にルーティング先を変える。

*   **ルール**: `payload.temperature > 30.0`
*   **実装**: 簡易的な式評価エンジン、または `Listener` 内でのユーザー定義ロジック。

---

## 3. エコシステムとの統合 (Generic traits)

Rust の標準的なエコシステムと連携し、車輪の再発明を防ぎます。

### A. `tower::Service` 統合
Rust の非同期ミドルウェアのデファクトスタンダードである `tower` と相互運用します。

*   **`Service` as `Handler`**: `tower::Service` を実装した既存のコンポーネント（例: タイムアウト、レートリミット、ロードバランサ）をそのまま `risten` のハンドラとして利用可能にする。
*   **メリット**: エコシステムの豊富なミドルウェア資産を即座に利用可能。

### B. `tracing` & `metrics` (Observability)
*   **Trace Context**: イベントヘッダからトレースID (OpenTelemetry) を抽出し、処理フロー全体の `Span` を自動生成。
*   **Metrics**: イベントのスループット、レイテンシ、**「Drop率（Bump Allocatorの効果測定）」** をPrometheus形式で公開する標準Hook。

---

## 4. プラグインシステム (WASM)

「コアエンジンはRustでコンパイル済み、ロジックは動的にロード」という構成を実現します。

*   **WASM Host**: `wasmtime` や `wasmer` を組み込む。
*   **Interface**: `wit-bindgen` 等を用いて、ホスト(Rust)とゲスト(WASM)の間でイベントデータを受け渡す標準インターフェースを定義。
*   **Sandbox**: プラグインが触れるメモリや機能を制限し、マルチテナント環境でも安全にコードを実行。
*   **用途**:
    *   ユーザー定義のフィルタリングロジック（ホットデプロイ可能）。
    *   FaaS (Function as a Service) 基盤の構築。

---

## まとめ: risten の進化形

`risten` は単なる「Botフレームワーク」ではなく、以下のような特徴を持つ **「高効率イベント処理カーネル」** へと進化できます。

1.  **Bump Allocation** による、フィルタリング・ルーティングフェーズの徹底的なゼロコスト化。
2.  **汎用ルーティング** による、あらゆるメッセージングプロトコルへの対応。
3.  **Tower / WASM 対応** による、無限の拡張性と運用性。

この設計により、Discord Bot はもちろん、IoTゲートウェイ、ログルーター、軽量Webサーバーなど、Rustのパフォーマンスが求められるあらゆるイベント駆動アプリケーションの基盤となり得ます。
