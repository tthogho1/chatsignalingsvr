# Rust WASM WebRTC Client

このプロジェクトは、Rust の WebAssembly (WASM) を使用して WebRTC ビデオチャットクライアントを実装しています。JavaScript クライアントと同じ機能を提供しながら、Rust の型安全性とパフォーマンスの利点を活用しています。

## 🚀 特徴

### ✨ WASM の利点

- **型安全性**: Rust の強力な型システム
- **パフォーマンス**: ネイティブに近い実行速度
- **メモリ安全**: ランタイムエラーの削減
- **クロスプラットフォーム**: どのモダンブラウザでも動作

### 🎯 実装機能

- **WebSocket 通信**: サーバーとのリアルタイム通信
- **WebRTC ビデオ通話**: P2P 音声・映像通話
- **テキストチャット**: ブロードキャスト・直接メッセージ
- **メディア制御**: カメラ・マイクのオン/オフ
- **DOM 操作**: ネイティブ Web API 連携

## 📋 必要条件

### ツール

- **Rust**: 1.70 以上
- **wasm-pack**: WASM ビルドツール
- **Python**: 3.7 以上（開発サーバー用）

### インストール

```bash
# Rust (既にインストール済みの場合はスキップ)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# または cargo でインストール
cargo install wasm-pack
```

## 🛠️ ビルドと実行

### 1. WASM モジュールのビルド

#### Windows:

```cmd
build-wasm.bat
```

#### Linux/macOS:

```bash
chmod +x build-wasm.sh
./build-wasm.sh
```

#### 手動ビルド:

```bash
wasm-pack build --target web --out-dir web-client/pkg --out-name wasm_client
```

### 2. サーバーの起動

#### WebSocket サーバー（ターミナル 1）:

```bash
cargo run
```

#### Web サーバー（ターミナル 2）:

```bash
cd web-client
python server.py
```

### 3. ブラウザでアクセス

`http://localhost:8000/wasm-index.html` を開いてください。

## 📁 プロジェクト構造

```
src/wasm_client/
├── lib.rs              # メインWASMエントリーポイント
├── websocket_client.rs # WebSocket通信
├── webrtc_client.rs    # WebRTC機能
├── dom_helpers.rs      # DOM操作ヘルパー
└── types.rs            # 型定義

web-client/
├── wasm-index.html     # WASM版HTML
├── styles.css          # 共通スタイル
├── pkg/                # ビルド出力（自動生成）
│   ├── wasm_client.js
│   ├── wasm_client_bg.wasm
│   └── ...
└── server.py           # 開発サーバー
```

## 🔧 アーキテクチャ

### WASM モジュール構造

```rust
VideoChat (メインクラス)
├── WebSocketClient (WebSocket通信)
├── WebRTCClient (WebRTC機能)
├── DomHelpers (DOM操作)
└── Types (共通型定義)
```

### JavaScript ↔ WASM 通信

```
JavaScript          WASM (Rust)
    │                   │
    ├─── 関数呼び出し ────→ VideoChat.method()
    │                   │
    ←─── Promise/結果 ─── async fn method()
    │                   │
    ├─── DOM Events ─────→ DOM操作
    │                   │
    ←─── Callbacks ───── コールバック
```

### WebRTC フロー

```
1. getUserMedia() ─→ MediaStream取得
2. createOffer() ─→ SDP Offer作成
3. WebSocket ─→ シグナリング送信
4. handleAnswer() ─→ SDP Answer処理
5. ICE交換 ─→ P2P接続確立
6. 音声・映像通信開始
```

## 🧪 デバッグ

### ブラウザ開発者ツール

```javascript
// WASMインスタンスにアクセス
const videoChat = window.videoChat;

// 手動で関数を呼び出し
await videoChat.connect('ws://127.0.0.1:8080', 'testuser');
await videoChat.start_call('otheruser');
```

### Rust コンソールログ

```rust
console::log_1(&"Debug message".into());
console::log_2(&"Key".into(), &"Value".into());
```

### パニックハンドリング

```rust
console_error_panic_hook::set_once();
```

## 🔄 JavaScript 版との比較

| 特徴           | JavaScript | Rust WASM |
| -------------- | ---------- | --------- |
| 型安全性       | ❌         | ✅        |
| パフォーマンス | 🟡         | ✅        |
| デバッグ       | ✅         | 🟡        |
| ビルド時間     | ✅         | 🟡        |
| バンドルサイズ | ✅         | 🟡        |
| 開発体験       | ✅         | 🟡        |

## ⚡ パフォーマンス最適化

### ビルド最適化

```bash
# リリースビルド
wasm-pack build --release --target web

# サイズ最適化
wasm-pack build --release --target web -- --features wee_alloc
```

### 実行時最適化

```rust
// メモリ使用量を削減
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
```

## 🚨 トラブルシューティング

### よくある問題

1. **WASM 読み込みエラー**

   ```
   Failed to compile WebAssembly module
   ```

   - ブラウザが WASM をサポートしているか確認
   - CORS 設定を確認（HTTPS が必要な場合）

2. **WebRTC 接続失敗**

   ```
   Failed to get user media
   ```

   - HTTPS で接続しているか確認（本番環境）
   - カメラ・マイクの許可を確認

3. **WebSocket 接続エラー**
   ```
   WebSocket connection failed
   ```
   - サーバーが起動しているか確認
   - URL が正しいか確認

### デバッグツール

```bash
# WASMバイナリの解析
wasm-objdump -x web-client/pkg/wasm_client_bg.wasm

# サイズ分析
wasm-opt --print-size web-client/pkg/wasm_client_bg.wasm
```

## 🔮 今後の改善

- [ ] **バンドルサイズ最適化**: 不要な機能の削除
- [ ] **エラーハンドリング**: より詳細なエラー情報
- [ ] **テスト**: WASM ユニットテストの追加
- [ ] **パフォーマンス**: メモリ使用量の最適化
- [ ] **機能拡張**: ファイル共有、画面共有
- [ ] **デプロイ**: CDN 対応、PWA 化

## 📚 参考資料

- [wasm-bindgen Book](https://rustwasm.github.io/wasm-bindgen/)
- [web-sys API Documentation](https://rustwasm.github.io/wasm-bindgen/api/web_sys/)
- [WebRTC API Guide](https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API)
- [Rust WASM Best Practices](https://rustwasm.github.io/book/)

## 📄 ライセンス

このプロジェクトは MIT ライセンスの下で公開されています。
