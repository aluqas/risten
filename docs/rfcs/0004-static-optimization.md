# RFC 0004: Static Optimization & Type System

- **Status**: Proposed
- **Type**: Optimization Track

## Summary
Rust の強力な型システムとコンパイラを活用して、パフォーマンスと安全性を極限まで高めるための技術提案。

## 1. Enum Dispatch (静的ポリモーフィズムの自動化)

Gatewayから流れてくる大量のイベント (MessageCreate, InteractionCreate...) を処理する際、`Box<dyn Hook>` (動的ディスパッチ) を使うと、毎回 vtable の参照が発生します。これを型レベルで解決します。

* **アイデア:** enum で定義されたイベント群に対して、マクロで自動的に Hook トレイトを実装させます。
* **効果:** match 文がコンパイル時に展開され、インライン化の恩恵をフルに受けられます。`enum_dispatch` クレートの Hook 版です。

```rust
// マクロで、このEnum自体が高速なDispatcherになる
#[risten::dispatch]
enum GatewayEvent {
    Message(MessageHook),
    Interaction(InteractionHook),
}
```

## 2. Const Generics による静的ルーティング

Prefixコマンドや固定パスのルーティングにおいて、文字列比較を実行時に行うのではなく、型レベルで定義します。

* **アイデア:** `const STR: &'static str` をジェネリクスに取るルーター型を用意します。
* **効果:** コンパイラが「"!ping"の場合の分岐」を静的に把握できるため、最適化が効きやすくなります。

```rust
// 型シグネチャにルーティング情報が含まれる
type MyRouter = Router<
    Route<"ping", PingHandler>, // "ping" という文字列が型情報になる
    Route<"echo", EchoHandler>,
>;
```

## 3. Typestate パターンによる「依存関係の静的保証」

「このハンドラはDB接続がないと動かない」といった制約を、実行時エラーではなくコンパイルエラーにします。

* **アイデア:** ハンドラが必要とするリソース（Extractor）を型レベルで表明し、Dispatcher構築時にそれが供給されているかチェックします。
* **効果:** 「Dependency Injection の失敗」がコンパイル時にわかります。

```rust
// ハンドラ定義: DBが必要だと型で主張
async fn user_handler(e: Event, db: Res<DbPool>) { ... }

// 構築時:
let dispatcher = Dispatcher::new()
    .provide(my_db_pool) // これを忘れると...
    .register(user_handler) // <--- ここでコンパイルエラー！
    .build();
```

## 4. HTree (Heterogeneous Tree) による分岐の静的化

現在の HList (線形リスト) を拡張し、型レベルで木構造 (HTree) を構築します。

* **アイデア:** HList が「次へ」しか持たないのに対し、HTree は「分岐」を持ちます。
* **実装イメージ:**

    ```rust
    // 型レベルで分岐ロジックを表現
    type MyPipeline = Branch<
        IsMessageCreate,   // 条件 (型レベル述語)
        MessagePipeline,   // Trueの分岐
        OtherPipeline      // Falseの分岐
    >;
    ```

* **効果:** 巨大なGatewayイベントの振り分け処理が、実行時の動的な登録処理を一切経ずに、単一の巨大な関数としてコンパイルされます。CPUの命令キャッシュ効率が最適化されます。

## 5. 理論上の限界への最適化ロードマップ

1. **最大の障壁：Message: 'static 制約の撤廃 (Zero-Copyへの道)**
    - **最適化案:** Listener レイヤーだけライフタイム (`'a`) を許可する設計へ変更。
    - **GAT (Generic Associated Types) の活用:** Dispatcher や Pipeline の定義を見直し、Listener の段階では `Message<'a>` を扱えるようにします。

2. **「巨大Match文」の自動生成 (Static Routingの具現化)**
    - **最適化案:** Proc-Macroによる トライ木(Trie) ルーターの生成。
    - コンパイル時計算により、登録された全コマンド（文字列）を解析し、最適な分岐コードを生成します。

3. **「枝刈り」の型レベル強制 (Data Control)**
    - **最適化案:** Extraction Trait の導入。
    - 誰も欲しがっていないフィールドは最初からパースしない（JSONデシリアライズすらしない）ことを可能にします。
