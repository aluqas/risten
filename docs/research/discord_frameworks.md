# Research: Discord Framework Comparison

Discord ボットフレームワークの詳細な比較分析です。

## 1. discord.py (Python)

discord.py は Python における Discord ボット開発のデファクトスタンダード。

### 特徴的な設計

```python
# デコレータベースのイベント登録
@bot.event
async def on_ready():
    print(f'{bot.user} has connected!')

@bot.event
async def on_message(message):
    if message.author == bot.user:
        return
    if message.content.startswith('!hello'):
        await message.channel.send('Hello!')

# 複数リスナーの登録
@bot.listen('on_message')
async def log_messages(message):
    print(f"Log: {message.content}")

# Cog によるモジュール化
class MyCog(commands.Cog):
    @commands.Cog.listener()
    async def on_message(self, message):
        pass

    @commands.command()
    async def greet(self, ctx):
        await ctx.send('Hello!')
```

### discord.py の強み

| 特徴               | 説明                                                          |
| ------------------ | ------------------------------------------------------------- |
| **デコレータ構文** | `@bot.event` で「ただ書けばいい」                             |
| **暗黙の登録**     | デコレートするだけで自動登録                                  |
| **Cog システム**   | 機能モジュール化、ホットリロード対応                          |
| **複数リスナー**   | `@bot.listen()` で同一イベントに複数登録                      |
| **wait_for**       | `await bot.wait_for('message', check=...)` でインタラクティブ |

### risten への示唆

```rust
// discord.py 風の体験を risten で
#[risten::event]
async fn on_ready(ready: &ReadyEvent) {
    println!("{} has connected!", ready.user.name);
}

#[risten::event]
async fn on_message(msg: &MessageEvent) {
    if msg.author.is_bot { return; }
    if msg.content.starts_with("!hello") {
        msg.reply("Hello!").await;
    }
}

#[risten::listen("on_message")]
async fn log_messages(msg: &MessageEvent) {
    println!("Log: {}", msg.content);
}
```

---

## 2. Serenity (Rust)

Serenity は Rust における主要な Discord ライブラリ。

### 特徴的な設計

```rust
// EventHandler trait 実装
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {:?}", why);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

// RawEventHandler for low-level access
#[async_trait]
impl RawEventHandler for Handler {
    async fn raw_event(&self, _ctx: Context, event: Event) {
        // 生イベントへのアクセス
    }
}
```

### Serenity の強み

| 特徴                        | 説明                                      |
| --------------------------- | ----------------------------------------- |
| **trait ベース**            | `EventHandler` 実装で型安全               |
| **メソッド名 = イベント名** | `async fn message()` = MESSAGE_CREATE     |
| **Context**                 | HTTP クライアント、キャッシュへのアクセス |
| **RawEventHandler**         | 低レベルアクセスも可能                    |
| **Framework**               | コマンドフレームワーク統合                |

### Serenity の課題

| 課題                 | 説明                           |
| -------------------- | ------------------------------ |
| **単一ハンドラ制約** | `EventHandler` は基本 1 つだけ |
| **フィルタリング**   | ハンドラ内で if 文で分岐       |
| **動的登録不可**     | コンパイル時に固定             |
| **マクロ必須**       | `#[async_trait]` が必要        |

### risten との比較

| 側面           | Serenity          | risten              |
| -------------- | ----------------- | ------------------- |
| 登録方式       | trait impl        | Registry / Static   |
| 複数ハンドラ   | ❌ 制限あり        | ✅ 無制限            |
| フィルタリング | ❌ ハンドラ内      | ✅ `Listener` で分離 |
| 動的登録       | ❌                 | ✅ Registry          |
| 静的最適化     | ❌                 | ✅ HList             |
| 低レベル       | ✅ RawEventHandler | ✅ Hook              |

---

## 3. Poise (Serenity 拡張)

Poise は Serenity 上に構築されたコマンドフレームワーク。

```rust
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or(ctx.author());
    ctx.say(format!("{}'s account was created at {}", user.name, user.created_at())).await?;
    Ok(())
}

// イベントハンドラ
async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            println!("Logged in as {}", data_about_bot.user.name);
        }
        _ => {}
    }
    Ok(())
}
```

### Poise の強み

| 特徴                 | 説明                         |
| -------------------- | ---------------------------- |
| **コマンドマクロ**   | `#[poise::command]` で宣言的 |
| **パラメータ解析**   | 関数引数から自動解析         |
| **slash + prefix**   | 両対応を一つの関数で         |
| **イベントハンドラ** | match でパターンマッチ       |

---

## 4. 統合比較マトリクス

### 4.1 言語間・フレームワーク特性比較

このマトリクスは、各フレームワークが「どの程度特定の機能をサポートしているか」を包括的に比較します。

| Framework         |  Lang  | Typed | Async | Static Opt. | Declarative | Pipeline | Middleware | State Mgmt  | Distributed |
| :---------------- | :----: | :---: | :---: | :---------: | :---------: | :------: | :--------: | :---------: | :---------: |
| **risten**        |  Rust  |  ✅✅   |   ✅   |      ✅      | ⚠️ _(Plan)_  |    ✅     | ⚠️ _(Plan)_ |      ❌      | ⚠️ _(Plan)_  |
| **discord.py**    | Python |   ❌   |   ✅   |      ❌      |      ✅      |    ❌     | ✅ (Checks) |   ⚠️ (Cog)   |      ❌      |
| **Serenity**      |  Rust  |   ✅   |   ✅   |      ❌      |      ❌      |    ❌     |     ❌      | ✅ (Context) |      ❌      |
| **Tower**         |  Rust  |   ✅   |   ✅   |      ❌      |      ❌      |    ✅     |     ✅      |      ❌      |      ❌      |
| **Actix**         |  Rust  |   ✅   |   ✅   |      ❌      |      ❌      |    ❌     |     ❌      |  ✅ (Actor)  |      ❌      |
| **Bevy ECS**      |  Rust  |   ✅   |   ❌   |      ✅      | ✅ (System)  |    ❌     |     ❌      |   ✅ (Res)   |      ❌      |
| **MediatR**       |   C#   |   ✅   |   ✅   |      ❌      |      ❌      |    ✅     |     ✅      |      ❌      |      ❌      |
| **RxJS**          |   TS   |   ✅   |   ✅   |      ❌      |      ❌      |    ✅     |     ✅      |      ⚠️      |      ❌      |
| **Spring Events** |  Java  |   ✅   |   ✅   |      ❌      |    ✅ (@)    |    ❌     |     ❌      |  ✅ (Bean)   |      ⚠️      |
| **Watermill**     |   Go   |   ✅   |   ✅   |      ❌      |      ❌      |    ✅     |     ✅      |      ❌      |      ✅      |
| **Kotlin Flow**   | Kotlin |   ✅   |   ✅   |      ❌      |      ❌      |    ✅     |     ❌      |  ✅ (State)  |      ❌      |
| **GenServer**     | Elixir |   ⚠️   |   ✅   |      ❌      |      ❌      |    ❌     |     ❌      |     ✅✅      |      ✅      |

#### 凡例

- **Static Opt.**: コンパイル時最適化 (Zero-cost abstraction)
- **Declarative**: アノテーションやマクロによる宣言的定義
- **Middleware**: 横断的関心事の注入メカニズム
- **State Mgmt**: フレームワーク自体が状態管理機能を提供するか
- **Distributed**: クラスタ/分散環境へのネイティブ対応

### 4.2 アーキテクチャ適合性

各フレームワークがどのアーキテクチャパターンに最適かを評価します。

| Framework       | Simple Event | Pipeline/Stream | CQRS/DDD | Microservices | UI/State | High Perf. |
| :-------------- | :----------: | :-------------: | :------: | :-----------: | :------: | :--------: |
| **risten**      |      ✅       |       ✅✅        | ✅ (Cmd)  |       ⚠️       |    ❌     |     ✅✅     |
| **discord.py**  |      ✅✅      |        ❌        |    ❌     |       ❌       |    ❌     |     ❌      |
| **Serenity**    |      ✅       |        ❌        |    ❌     |       ❌       |    ❌     |     ✅      |
| **Watermill**   |      ❌       |        ✅        |    ✅✅    |      ✅✅       |    ❌     |     ✅      |
| **MediatR**     |      ✅       |        ✅        |    ✅✅    |       ❌       |    ❌     |     ✅      |
| **RxJS**        |      ✅       |       ✅✅        |    ❌     |       ❌       |    ✅✅    |     ❌      |
| **Kotlin Flow** |      ✅       |       ✅✅        |    ❌     |       ❌       |    ✅✅    |     ✅      |
