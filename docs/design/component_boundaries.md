# Component Boundaries

イベント処理パイプラインにおける各コンポーネントの責務境界の定義です。

## 1. 境界の再定義：3つのステップ

イベント処理の流れは、以下の3ステップに分解されます。

1. **Routing (Selection):** 「このイベント、誰（どのHookたち） が欲しがってる？」
    * **入力:** Event
    * **出力:** `Iterator<Item = &Hook>` (処理すべきHookのリスト)
    * **役割:** Registry や Router が担当。条件マッチした Hook だけを返します。

2. **Delivery (Execution Strategy):** 「そのHookたちに、どういう順序・並列度 でイベントを渡す？」
    * **入力:** Event, `Iterator<Item = &Hook>`
    * **出力:** `Result` (完了/中断)
    * **役割:** 「直列」「並列(Fanout)」「遅延」などの実行戦略を決定します。

3. **Dispatcher (Orchestrator):** 上記2つを束ねる**「監督」**
    * **役割:** ルーターからHookを受け取り、デリバリー戦略に従って実行させます。

## 2. 構造定義

```rust
struct StandardDispatcher<R, D> {
    router: R,      // Routing: 誰に？ (Registry or HashMapRouter or Matchit)
    delivery: D,    // Delivery: どうやって？ (Sequential or Parallel)
}

impl<R, D> Dispatcher for StandardDispatcher<R, D> {
    async fn dispatch(&self, event: E) {
        // 1. Ask Router
        let hooks = self.router.route(&event);

        // 2. Delegate to Delivery
        self.delivery.deliver(hooks, event).await;
    }
}
```

## 3. "Router" と "Listener" の関係性

「Routerも結局はListenerの一種ではないか？」 という視点に基づき、Router を Hook/Listener として実装するパターンを許容します。

* **Listener:** イベントを見て、「通す/通さない」や「変換する」を行う。
* **Router:** イベントを見て、「こっちのパイプラインに通す」を行う。

**RouterListener:** イベントを受け取り、内部のマップを見て、適切な子Listener（Handler）を実行して終了する「ネストできるListener」としての Router です。
