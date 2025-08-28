use base64::Engine;
use defmt::info;
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_time::Duration;
use embedded_io_async::{Read, Write};
use serde::Deserialize;

const INDEX_HTML: &str = include_str!("..\\assets\\index.html");

#[derive(Deserialize)]
struct ClickEvent {
    x: u8,
    y: u8,
    on: bool,
}
static mut RX_BUF: [u8; 1024] = [0_u8; 1024];
static mut TX_BUF: [u8; 1024] = [0_u8; 1024];

#[embassy_executor::task]
pub async fn web_server_task(stack: Stack<'static>) {
    defmt::info!("Starting web server...");
    const PORT: u16 = 8080;
    let mut http_buffer = [0; 2048];

    loop {
        #[allow(static_mut_refs)]
        let mut socket = TcpSocket::new(stack, unsafe { &mut RX_BUF }, unsafe { &mut TX_BUF });
        socket.set_timeout(Some(Duration::from_secs(120)));
        socket.set_keep_alive(Some(Duration::from_secs(60)));

        if socket.accept(PORT).await.is_err() {
            continue;
        }

        let request_length = match socket.read(&mut http_buffer).await {
            Ok(n) => n,
            Err(_) => continue,
        };
        let request = core::str::from_utf8(&http_buffer[..request_length]).unwrap_or("");

        if request.starts_with("GET /ws ") && request.contains("Upgrade: websocket") {
            let key = request
                .lines()
                .find(|l| l.starts_with("Sec-WebSocket-Key:"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim());
            let key = match key {
                Some(v) => v,
                None => continue,
            };
            let hash = {
                use sha1::{Digest, Sha1};
                let mut hasher = Sha1::new();
                hasher.update(key);
                hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
                base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
            };
            let response = alloc::format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {hash}\r\n\r\n");
            if socket.write_all(response.as_bytes()).await.is_err() {
                continue;
            }
            send_update(&mut socket).await;
            handle_ws(&mut socket).await;
        } else {
            let response = alloc::format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: text/html\r\n\
                Content-Length: {}\r\n\r\n\
                {}",
                INDEX_HTML.len(),
                INDEX_HTML
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
        socket.flush().await.unwrap();
        socket.close();
    }
}

async fn handle_ws(socket: &mut TcpSocket<'static>) {
    defmt::warn!("handle_ws");
    let mut payload = [0_u8; 128];
    let mut header = [0_u8; 6];

    loop {
        if socket.read_exact(&mut header[..2]).await.is_err() {
            break;
        };
        let op_code = header[0] & 0b00001111;
        if op_code == 9 {
            send_pong().await;
        } else if op_code != 1 {
            defmt::error!("Got socket message which is not text type.");
            break;
        }
        if header[1] >> 7 != 1 {
            defmt::error!("Got unmasked message from client.");
            break;
        }
        let payload_len = header[1] & 0b01111111;
        if payload_len > 125 {
            defmt::error!("Payload too long");
            break;
        }
        if socket.read_exact(&mut header[2..6]).await.is_err() {
            break;
        }
        let mask = &header[2..6];
        if socket
            .read_exact(&mut payload[..payload_len.into()])
            .await
            .is_err()
        {
            break;
        }
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4]
        }
        // Debug
        let x = &payload[..payload_len.into()];
        let y: alloc::string::String = x.utf8_chunks().map(|c| c.valid()).collect();
        info!("{}", y.as_str());
        //
        if let Ok((data, _)) =
            serde_json_core::from_slice::<ClickEvent>(&payload[..payload_len.into()])
        {
            crate::led_matrix::set(data.x, data.y, data.on);
            send_update(socket).await;
            defmt::info!("Set {}, {} to {}", data.x, data.y, data.on);
        } else {
            defmt::warn!("Failed to deserialize message from socket");
        }
    }
}

async fn send_update(socket: &mut TcpSocket<'static>) {
    let header = [0b10000010, 0b00001000];
    socket.write_all(&header).await.ok();
    socket
        .write_all(&crate::led_matrix::snapshot().to_be_bytes())
        .await
        .ok();
    socket.flush().await.unwrap();
}

async fn send_pong() {}
