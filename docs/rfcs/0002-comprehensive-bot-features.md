# RFC 0002: Comprehensive Bot Feature Set

- **Status**: Proposed
- **Type**: Feature Track

## Summary
大規模サーバー運用に求められる機能を網羅的に提供する「統合 Bot システム」の機能要件定義。
既存のバラバラな Bot（管理用、音楽用、統計用など）を一つの整合性のあるシステムとして再実装することを目指す。

## Motivation
現在の Discord Bot エコシステムは機能ごとに分断されており、ユーザー体験（UX）や設定の一貫性が欠如している。Rust のパフォーマンスを活かした「最強」の統合環境を提供する。

## Proposed Features

### 1. Core Modules (主要モジュール)
- **Saphire**: 汎用機能群（便利ツールなど）。
- **Vortex / Wick Alternative**: 高度なスパム対策・セキュリティ機能。
- **Web Dashboard**: 設定・管理用の Web インターフェース（検討中）。

### 2. Specialized Features (個別機能)
| 機能名 | 概要 | 備考 |
| :--- | :--- | :--- |
| **RolePanel** | ロール付与・管理 UI | ボタン/メニューによるインタラクティブな管理 |
| **Gatekeeper** | 入室管理・検証 | 信用スコアに基づくフィルタリング |
| **StatBot** | 統計情報収集・表示 | アクティビティ、参加推移など |
| **Status** | サーバー状態表示 | チャンネル名や Embed でのリアルタイム表示 |
| **Invite** | 招待追跡・管理 | 誰が誰を招待したかのトラッキング |
| **VoiceMaster** | ボイスチャンネル管理 | 動的なチャンネル生成 (Temp Voice) |
| **TicketTools** | チケットシステム | 問い合わせ対応フローの統合 |
| **EmbedGenerator** | 埋め込み生成 | リッチなメッセージ作成ツール |
| **Pajamyboard** | ピン留め・ハイライト | Starboard の高機能版 |
| **Pluralkit** | 複数人格支援 | 互換性または連携機能 |

### 3. Advanced Ideas (新機能アイデア)
- **TimeMachine**: (詳細未定) 過去の状態へのロールバックやログ参照？
- **AudioBridge**: 外部サービスや他チャンネルとの音声連携。
- **Automation (n8n-like)**:
  - イベントとアクションを GUI で繋ぐマクロ機能。
  - プログラミングレスでの自動化。
- **Incident Mode (緊急事態モード)**:
  - 荒らし発生時などに、ワンコマンドで「低速化」「招待停止」「厳格化」を一括適用するモード。
- **Doctor / Diagnose**:
  - `/diagnose` コマンドによるサーバー設定や権限の健全性診断。

## Integration Strategy (統合戦略)
- **モジュール性**: 各機能はオプトイン（選択可能）とし、不要な機能は無効化してリソースを節約できる設計とする。
- **UX 統一**: すべての機能で統一された UI キット（色、アイコン、メッセージフォーマット）を使用する。
