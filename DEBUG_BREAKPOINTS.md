# デバッグ推奨ブレークポイント

## ユーザ名登録のデバッグポイント

### 1. メッセージルーティング時のユーザ名抽出

**ファイル**: `src/lib.rs`
**行**: 483-486

```rust
// Update client username if provided in message sender_id
if let Some(username) = &message.sender_id {
    self.connection_manager.update_client_username(&client_id.to_string(), username.clone()).await;
}
```

### 2. ユーザ名による ClientId 検索

**ファイル**: `src/handlers/signaling.rs`
**行**: 31-38

```rust
// Look up the target client ID by username
let target_client_id = self.find_client_by_username(&target_username).await
    .ok_or_else(|| {
        warn!(
            sender_id = %sender_id,
            target_username = %target_username,
            "Target user not found by username for signaling message"
        );
        ConnectionError::ClientNotFound(target_username.clone())
    })?;
```

### 3. クライアント接続時

**ファイル**: `src/lib.rs`
**行**: 247-252

```rust
// Connect the client and get their ID and message receiver
let (client_id, message_receiver) = self.connection_manager.connect_client().await;

// Also add the client to the shared registry for message handlers
if let Some(client) = self.connection_manager.get_client(&client_id).await {
    self.clients.write().await.insert(client_id.clone(), client);
}
```

## デバッグ時の確認ポイント

### メッセージ受信時

- `message.sender_id` にユーザ名が含まれているか
- `client_id` と `username` の関連付けが正しいか

### WebRTC シグナリング時

- `target_username` が正しく受信されているか
- ユーザ名から ClientId への解決が成功しているか
- エラーハンドリングが適切に動作しているか

### ログ出力の確認

デバッグコンソールで以下のログを確認：

- `"Client username updated"` - ユーザ名登録成功
- `"Resolved username to client ID for signaling"` - ユーザ名解決成功
- `"Target user not found by username"` - ユーザ名が見つからない場合
