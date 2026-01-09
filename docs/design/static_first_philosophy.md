# Static-First Architecture 設計原則

> risten は **Static-First** の設計哲学を採用しています。
> 静的処理がデフォルト、動的処理は明示的なエスケープハッチとして提供されます。

---

## 1. 静的処理（デフォルトパス）

静的パスは **ゼロコスト抽象化** を実現し、コンパイル時に全ての最適化が行われます。

```rust
use risten::{static_hooks, StaticDispatcher, HCons, HNil};

// 静的Hookチェーン（ゼロコスト、インライン化）
let chain = static_hooks![LoggingHook, MetricsHook, MyHandler];
let dispatcher = StaticDispatcher::new(chain);
```

### 特徴

- **vtable なし**: 型レベルでチェーン構造が定義される
- **インライン化**: コンパイラが全てのHook呼び出しをインライン展開
- **CPUキャッシュ効率**: 単一の巨大関数として最適化される

### いつ使うか

- Hook の構成がコンパイル時に確定している場合
- 最大パフォーマンスが求められる場合（ほとんどのケース）
- 本番環境のイベントパイプライン

---

## 2. 動的処理（エスケープハッチ）

動的パスは `risten::dynamic` モジュールで明示的に提供されます。

```rust
use risten::dynamic::{DynamicDispatcher, RegistryBuilder};

// 動的Registry（ランタイム登録）
let registry = RegistryBuilder::new()
    .register(MyHook)
    .build();
```

### 特徴

- **ランタイム登録**: Hook の追加・削除が実行時に可能
- **trait object**: `Box<dyn Hook>` による動的ディスパッチ
- **柔軟性**: 構成が実行時まで確定しないシナリオに対応

### いつ使うか

- **プラグインシステム**: 外部から動的にロードされるHook
- **ホットリロード**: 設定変更時にHookを差し替える
- **開発・デバッグ**: 実行時のHook有効化/無効化

---

## 3. モジュール構造

```text
risten::                       ← 静的型がデフォルト
├── StaticDispatcher           ← 推奨
├── StaticFanoutDispatcher     ← 並列静的チェーン
├── HCons / HNil               ← HList構築ブロック
├── static_hooks!              ← 便利マクロ
│
├── dynamic::                  ← 明示的opt-in
│   ├── DynamicDispatcher      ← ランタイム柔軟性
│   ├── RegistryBuilder        ← 動的登録
│   └── EnabledHandle          ← 有効化制御
│
└── SimpleDynamicDispatcher    ← 旧SimpleDispatcher（便利エイリアス）
```

---

## 4. 移行ガイド

### v0.1 から v0.2 への移行

| v0.1                 | v0.2                               |
| -------------------- | ---------------------------------- |
| `StandardDispatcher` | `dynamic::DynamicDispatcher`       |
| `SimpleDispatcher`   | `SimpleDynamicDispatcher`          |
| `RegistryBuilder`    | `dynamic::RegistryBuilder`         |
| `static_dispatch::*` | crate root (e.g., `risten::HCons`) |

---

_最終更新: 2026-01-04_
