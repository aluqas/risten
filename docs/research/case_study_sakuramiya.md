# Case Study: Sakuramiya Discord Bot

「Sakuramiya Discord Bot」（超高性能・省メモリBot）を例に、アロケーション排除と動的処理の排除を `risten` でどう実現するかを考察します。

## 1. アロケーション排除と静的化の事例

Bot開発では「とりあえず String」「とりあえず Clone」「とりあえず Arc」で逃げがちですが、これを徹底的に **「Borrow (参照)」** と **「Static (静的)」** に置き換えるアプローチです。

### 1. Ingress: JSONデシリアライズとイベント定義

* **現状の課題 (Typical):** WebSocketから受け取ったJSONを、一旦全部 String を持つ構造体 (`serde_json::Value` や `discord_model::Message`) に変換してしまう。ここで大量のヒープ割り当てが発生します。
* **Sakuramiya流の解決策:** **「Zero-Copy Deserialization」** を採用します。
  * **実装:** イベント構造体は String ではなく `Cow<'a, str>` や `&'a str` を持ち、受信バッファ（WebSocketの生バイト列）を直接参照します。

    ```rust
    // 生のバッファを参照するイベント定義
    struct MessageCreate<'a> {
        content: &'a str, // バッファへの参照 (アロケーションなし)
        author_id: u64,
    }
    ```

  * **sakuramiya-eventでの扱い:** `Message` トレイトは `Send + Sync + 'static` を要求するため、ここだけ少し工夫（ライフタイムの壁）が必要ですが、Listener の段階までは生のバッファ (`&[u8]`) を回し、必要な場合だけ ToOwned する戦略（Lazy Parsing）が有効です。

### 2. Dispatch: イベント種別の判定

* **現状の課題:** `Box<dyn EventHandler>` のリストを回したり、イベント型ごとに HashMap を引いたりする（動的ディスパッチ、ポインタ参照）。
* **Sakuramiya流の解決策:** **「Enum Dispatch & Huge Match」** です。
  * **実装:** 全てのイベントを一つの巨大な enum `GatewayEvent` にまとめ、match 文で分岐します。

    ```rust
    // マクロで自動生成されるイメージ
    enum GatewayEvent<'a> {
        Message(MessageCreate<'a>),
        Interaction(InteractionCreate<'a>),
        // ...
    }
    ```

  * **sakuramiya-eventでの扱い:** これを StaticDispatcher に食わせます。コンパイラは match 文をジャンプテーブルに最適化するため、分岐コストはほぼゼロです。

### 3. Routing: コマンドと引数の解析

* **現状の課題:** `msg.content.split_whitespace().map(|s| s.to_string()).collect::<Vec<String>>()` のように、コマンド引数を解析するたびに String の配列を作ってしまう。
* **Sakuramiya流の解決策:** **「Zero-Copy Parser Listener」** です。
  * **実装:** Listener の中で、文字列をスライス (`&str`) のまま扱います。

    ```rust
    struct CommandParser;

    // 中間生成物（ハンドラに渡すデータ）も参照のみ
    struct CommandContext<'a> {
        command: &'a str,
        args: impl Iterator<Item = &'a str>, // イテレータとして渡す
    }

    impl Listener<MessageCreate<'_>> for CommandParser {
        type Output = CommandContext<'static>; // 実際はライフタイム管理が必要

        fn listen(&self, event: &MessageCreate) -> Option<Self::Output> {
            // 文字列操作はすべてスライス上で行う（アロケーションゼロ）
            let mut parts = event.content.split_whitespace();
            let cmd = parts.next()?;

            if cmd == "!ping" {
                Some(CommandContext { command: cmd, args: parts })
            } else {
                None
            }
        }
    }
    ```

### 4. Handler: 実行と状態管理

* **現状の課題:** ハンドラ呼び出し時に `Pin<Box<dyn Future>>` (ヒープ割り当て) が発生する。また、DB接続などを `Arc<Mutex<..>>` で共有してロック競合する。
* **Sakuramiya流の解決策:** **「Static Future & Lock-Free」** です。
  * **実装:**
    * RpITIT: sakuramiya-event は既に `impl Future` を返せるため、Box化は不要です（静的ディスパッチの場合）。
    * State: `Arc` のクローンを避けるため、Listener から Handler へは **「参照」** を渡したいところですが、非同期境界 (await を跨ぐ) があるため、ここは Arc が必要コストになります。
    * ただし、**「本当に必要なデータだけ」** を抽出してから渡すことで、巨大なContextのコピーを防げます。

## まとめ：Sakuramiya Botの構成図

```text
[WebSocket Buffer]  <-- (1) 生データ
       |
[Raw Event Parser]  <-- (2) Listener: JSONを読みつつ、興味あるフィールドだけ &str で抜く
       |                    (Allocation: 0)
       v
[Static Router]     <-- (3) Listener: "!ping" かどうかを &str 比較で分岐
       |                    (Allocation: 0, Dynamic Dispatch: 0)
       v
[CommandHandler]    <-- (4) Handler: 必要なデータ(Arc)と引数(&str)を受け取って実行
                            (Allocation: 最小限のFuture stateのみ)
```
