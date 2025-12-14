use bytes::{BufMut, BytesMut};
use quinn::Connection;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn send(conn: &Connection, msg: &[u8]) {
    let (mut send, _) = conn.open_bi().await.unwrap();

    let mut buf = BytesMut::with_capacity(4 + msg.len());
    buf.put_u32(msg.len() as u32);
    buf.extend_from_slice(msg);

    send.write_all(&buf).await.unwrap();
    send.finish().await.unwrap(); // async in quinn 0.10
}

pub async fn handle_streams(conn: Connection) {
    while let Ok((_send, mut recv)) = conn.accept_bi().await {
        let len = recv.read_u32().await.unwrap();
        let mut data = vec![0u8; len as usize];
        recv.read_exact(&mut data).await.unwrap();

        println!(
            "[RECV {}] {}",
            conn.remote_address(),
            String::from_utf8_lossy(&data)
        );
    }
}
