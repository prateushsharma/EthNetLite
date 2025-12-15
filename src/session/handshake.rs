use crate::session::message::{Hello, HelloAck, SessionMessage};
use crate::session::state::PeerSession;
use quinn::Connection;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn intersect_caps(local: &[String], remote: &[String]) -> Vec<String> {
    let mut out = vec![];
    for c in local {
        if remote.iter().any(|r| r == c) {
            out.push(c.clone());
        }
    }
    out
}

/// Send one framed message over a fresh bi-stream (length-prefixed)
async fn send_frame(conn: &Connection, payload: &[u8]) -> Result<(), ()> {
    let (mut send, _) = conn.open_bi().await.map_err(|_| ())?;
    send.write_u32(payload.len() as u32).await.map_err(|_| ())?;
    send.write_all(payload).await.map_err(|_| ())?;
    send.finish().await.map_err(|_| ())?;
    Ok(())
}

/// Read one framed message from an accepted bi-stream
async fn read_frame(recv: &mut quinn::RecvStream) -> Result<Vec<u8>, ()> {
    let len = recv.read_u32().await.map_err(|_| ())?;
    let mut data = vec![0u8; len as usize];
    recv.read_exact(&mut data).await.map_err(|_| ())?;
    Ok(data)
}

/// Outbound handshake: we dial someone and initiate Hello, wait for HelloAck.
pub async fn outbound_handshake(
    conn: &Connection,
    local_node_id: &str,
    local_caps: &[String],
) -> Result<PeerSession, ()> {
    let hello = Hello {
        node_id: local_node_id.to_string(),
        protocol_version: 1,
        capabilities: local_caps.to_vec(),
        genesis: "0xgenesis".to_string(),
        head_height: 0,
    };

    send_frame(conn, &SessionMessage::Hello(hello).to_bytes()).await?;

    // Wait for ack by accepting an inbound stream (peer will open one)
    let (_send, mut recv) = conn.accept_bi().await.map_err(|_| ())?;
    let data = read_frame(&mut recv).await?;

    match SessionMessage::from_bytes(&data) {
        Some(SessionMessage::HelloAck(ack)) => Ok(PeerSession {
            remote_node_id: ack.node_id,
            agreed_caps: ack.agreed_capabilities,
        }),
        _ => Err(()),
    }
}

/// Inbound handshake handler: when someone connects to us, they send Hello.
pub async fn inbound_handshake(
    conn: &Connection,
    local_node_id: &str,
    local_caps: &[String],
) -> Result<PeerSession, ()> {
    // Wait for their Hello
    let (_send, mut recv) = conn.accept_bi().await.map_err(|_| ())?;
    let data = read_frame(&mut recv).await?;

    let remote_hello = match SessionMessage::from_bytes(&data) {
        Some(SessionMessage::Hello(h)) => h,
        _ => return Err(()),
    };

    let agreed = intersect_caps(local_caps, &remote_hello.capabilities);

    // Reply with HelloAck
    let ack = HelloAck {
        node_id: local_node_id.to_string(),
        agreed_capabilities: agreed.clone(),
    };

    send_frame(conn, &SessionMessage::HelloAck(ack).to_bytes()).await?;

    Ok(PeerSession {
        remote_node_id: remote_hello.node_id,
        agreed_caps: agreed,
    })
}
