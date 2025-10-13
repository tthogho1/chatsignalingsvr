# Chat Signaling Server

WebSocket ベースのシグナリングサーバーと WebRTC ビデオチャットクライアントです。

## 🚀 機能

### サーバー側

- **WebSocket サーバー**: リアルタイム通信
- **メッセージルーティング**: ブロードキャスト・直接メッセージ
- **WebRTC シグナリング**: offer/answer/ICE candidate の配信
- **設定管理**: 環境変数ベースの設定
- **ログ機能**: 構造化ログとトレーシング

### クライアント側

- **CLI クライアント**: コマンドライン操作
- **Web クライアント**: ブラウザベースのビデオチャット
- **ライブラリ**: 再利用可能なクライアントライブラリ

## 📋 必要条件

- **Rust**: 1.70 以上
- **Python**: 3.7 以上（Web クライアント用）
- **モダンブラウザ**: WebRTC 対応（Chrome、Firefox、Safari、Edge）

## ⚡ クイックスタート

### 1. プロジェクトのクローン・セットアップ

```bash
git clone <repository-url>
cd chatsignalingsvr
```

### 2. サーバーの起動

```bash
# デフォルト設定で起動
cargo run

# または設定を指定
CHAT_SERVER_HOST=0.0.0.0 CHAT_SERVER_PORT=8080 cargo run
```

### 3. Web クライアントの起動

```bash
cd web-client
python server.py
```

### 4. ブラウザでアクセス

`http://localhost:8000` をブラウザで開いて、ビデオチャットをお楽しみください！

## 🛠️ 詳細な使用方法

### サーバー設定

環境変数で設定をカスタマイズできます：

```bash
# サーバーのホストとポート
export CHAT_SERVER_HOST=127.0.0.1
export CHAT_SERVER_PORT=8080

# ログレベル
export RUST_LOG=info

# 起動
cargo run
```

### CLI クライアントの使用

```bash
# インタラクティブモード
cargo run --bin client

# コマンドライン引数で実行
cargo run --bin client -- --url ws://127.0.0.1:8080 --username myuser --command "/broadcast Hello World"
```

#### CLI コマンド

- `/broadcast <message>`: 全員にメッセージを送信
- `/direct <username> <message>`: 特定ユーザーにメッセージを送信
- `/signal <target> <type> <data>`: WebRTC シグナリングデータを送信
- `/quit`: 接続を終了

### Web クライアントの使用

詳細は [web-client/README.md](web-client/README.md) を参照してください。

## 🏗️ アーキテクチャ

### プロジェクト構造

```
├── Cargo.toml              # プロジェクト設定
├── src/
│   ├── main.rs             # サーバーエントリーポイント
│   ├── lib.rs              # ライブラリルート
│   ├── config.rs           # 設定管理
│   ├── logging.rs          # ログ設定
│   ├── handlers/           # WebSocketハンドラー
│   │   ├── connection.rs   # 接続管理
│   │   ├── message.rs      # メッセージ処理
│   │   └── signaling.rs    # WebRTCシグナリング
│   ├── models/             # データモデル
│   │   ├── client.rs       # クライアントモデル
│   │   ├── message.rs      # メッセージモデル
│   │   └── config.rs       # 設定モデル
│   ├── client/             # クライアントライブラリ
│   └── bin/
│       └── client.rs       # CLIクライアント
├── web-client/             # Webクライアント
│   ├── index.html
│   ├── styles.css
│   ├── webrtc-client.js
│   └── server.py
├── examples/               # サンプルコード
├── tests/                  # テスト
└── docs/                   # ドキュメント
```

### メッセージフロー

```
[Webクライアント] <--WebSocket--> [サーバー] <--WebSocket--> [他のクライアント]
                                      |
                                 [メッセージ]
                                 [ルーティング]
                                      |
                            [ブロードキャスト/直接送信]
```

### WebRTC シグナリング

```
[クライアントA] --offer--> [サーバー] --offer--> [クライアントB]
[クライアントA] <-answer-- [サーバー] <-answer-- [クライアントB]
[クライアントA] <-ICE候補-> [サーバー] <-ICE候補-> [クライアントB]
                           |
                    [P2P接続確立]
                           |
                   [直接音声・映像通信]
```

## 🧪 テスト

### 単体テスト

```bash
cargo test
```

### 統合テスト

```bash
cargo test --test integration_tests
```

### 設定テスト

```bash
cargo test config
```

## 📚 API リファレンス

### WebSocket メッセージ

#### クライアント → サーバー

```json
{
  "type": "broadcast",
  "content": "Hello everyone!",
  "sender": "username"
}
```

```json
{
  "type": "direct",
  "content": "Private message",
  "sender": "username",
  "target": "recipient"
}
```

```json
{
  "type": "signaling",
  "signaling_type": "offer",
  "data": "WebRTC SDP offer",
  "sender": "username",
  "target": "recipient"
}
```

#### サーバー → クライアント

```json
{
  "type": "broadcast",
  "content": "Hello everyone!",
  "sender": "username",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

```json
{
  "type": "signaling",
  "signaling_type": "offer",
  "data": "WebRTC SDP offer",
  "sender": "username",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

## 🐛 トラブルシューティング

### よくある問題

1. **サーバーが起動しない**

   ```bash
   # ポートが使用中の場合
   CHAT_SERVER_PORT=8081 cargo run
   ```

2. **クライアントが接続できない**

   ```bash
   # サーバーが起動しているか確認
   ps aux | grep chatsignalingsvr

   # ポートが開いているか確認
   netstat -an | grep 8080
   ```

3. **WebRTC 接続が失敗する**
   - HTTPS を使用しているか確認（本番環境）
   - ファイアウォール設定を確認
   - STUN サーバーの動作を確認

### ログの確認

```bash
# デバッグログを有効にして起動
RUST_LOG=debug cargo run
```

## 🚀 デプロイ

### 本番環境

```bash
# リリースビルド
cargo build --release

# 実行
CHAT_SERVER_HOST=0.0.0.0 \
CHAT_SERVER_PORT=8080 \
RUST_LOG=info \
./target/release/chatsignalingsvr
```

### Docker（オプション）

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/chatsignalingsvr /usr/local/bin/
EXPOSE 8080
CMD ["chatsignalingsvr"]
```

## 🤝 コントリビューション

1. フォークしてください
2. フィーチャーブランチを作成: `git checkout -b feature/amazing-feature`
3. 変更をコミット: `git commit -m 'Add amazing feature'`
4. ブランチにプッシュ: `git push origin feature/amazing-feature`
5. プルリクエストを開いてください

## 📄 ライセンス

このプロジェクトは MIT ライセンスの下で公開されています。詳細は [LICENSE](LICENSE) ファイルを参照してください。

## 📞 サポート

- 問題や質問がある場合は [Issues](../../issues) を確認
- 新しい機能の提案は [Discussions](../../discussions) で議論
- バグレポートは詳細な情報と再現手順を含めてください

## 🎯 今後の計画

- [ ] ユーザー認証システム
- [ ] ファイル共有機能
- [ ] 画面共有機能
- [ ] モバイルアプリ対応
- [ ] グループビデオ通話
- [ ] メッセージ暗号化
- [ ] データベース連携
