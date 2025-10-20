# WebSocket Chat and Signaling Server - 起動ガイド

## サーバー起動用バッチファイル

このプロジェクトには、サーバーを簡単に起動するための複数のバッチファイルが用意されています。

### 📁 利用可能なバッチファイル

| ファイル名                    | 用途           | 設定                       |
| ----------------------------- | -------------- | -------------------------- |
| `start-server.bat`            | **基本起動**   | localhost:8080、info ログ  |
| `start-server-dev.bat`        | **開発用**     | 0.0.0.0:8080、debug ログ   |
| `start-server-production.bat` | **本番用**     | 0.0.0.0:8080、最適化ビルド |
| `run-tests.bat`               | **テスト実行** | 全テストスイートの実行     |

### 🚀 基本的な使用方法

#### 1. 基本起動（localhost のみ）

```batch
.\start-server.bat
```

- **用途**: ローカル開発・テスト
- **アクセス**: `ws://127.0.0.1:8080`
- **設定**: localhost バインド、info ログレベル

#### 2. 開発用起動（ネットワークアクセス可能）

```batch
.\start-server-dev.bat
```

- **用途**: チーム開発、デバイステスト
- **アクセス**: `ws://127.0.0.1:8080` または `ws://[your-ip]:8080`
- **設定**: 全インターフェース、debug ログレベル

#### 3. 本番用起動（最適化済み）

```batch
.\start-server-production.bat
```

- **用途**: 本番環境、パフォーマンステスト
- **アクセス**: `ws://127.0.0.1:8080` または `ws://[your-ip]:8080`
- **設定**: Release ビルド、warn ログレベル、高い接続数制限

### 🧪 テスト実行

```batch
.\run-tests.bat
```

- 単体テスト、統合テスト、ドキュメントテストを実行

### ⚙️ 詳細設定

各バッチファイルの設定:

#### start-server.bat

- Bind Address: `127.0.0.1`
- Port: `8080`
- Max Connections: `1000`
- Log Level: `info`

#### start-server-dev.bat

- Bind Address: `0.0.0.0`
- Port: `8080`
- Max Connections: `100`
- Log Level: `debug`

#### start-server-production.bat

- Bind Address: `0.0.0.0`
- Port: `8080`
- Max Connections: `2000`
- Log Level: `warn`
- Build Mode: `--release`

### 🔧 カスタム設定

環境変数で設定を上書きできます:

```batch
REM 環境変数で設定
set SERVER_BIND_ADDRESS=0.0.0.0
set SERVER_PORT=9090
set MAX_CONNECTIONS=500
set LOG_LEVEL=debug

REM サーバー起動
.\start-server.bat
```

### 🛠️ 手動起動

バッチファイルを使わずに手動で起動する場合:

```batch
REM デバッグビルド
cargo run --bin websocket-chat-signaling-server

REM カスタム設定
cargo run --bin websocket-chat-signaling-server -- --bind-address 0.0.0.0 --port 8080 --log-level debug

REM リリースビルド
cargo build --release --bin websocket-chat-signaling-server
target\release\websocket-chat-signaling-server.exe --bind-address 0.0.0.0 --port 8080
```

### 🚨 トラブルシューティング

1. **Rust がインストールされていない**

   - https://rustup.rs/ からインストール

2. **ポートが使用中**

   - 別のポートを指定: `--port 8081`

3. **バインドエラー**

   - 管理者権限で実行
   - ファイアウォール設定を確認

4. **ビルドエラー**
   - `cargo clean` で一旦クリーンアップ
   - 依存関係を更新: `cargo update`

### 📝 ログ出力

サーバーは構造化ログ（JSON 形式）を出力します:

- 接続/切断イベント
- メッセージルーティング
- エラー情報
- パフォーマンス情報

ログレベルによる出力内容:

- `error`: エラーのみ
- `warn`: 警告以上
- `info`: 情報以上（推奨）
- `debug`: デバッグ情報（開発時）
- `trace`: 全ての詳細情報

### 🔗 関連ファイル

- **WebClient**: `web-client/server.py` - フロントエンド用 HTTP サーバー
- **WASM Build**: `build-wasm.bat` - WASM クライアントビルド
- **Configuration**: `.env.sample` - 環境変数設定例
