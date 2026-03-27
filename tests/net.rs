use nene::net::{Client, ClientEvent, Server, ServerEvent};
use std::time::{Duration, Instant};

fn wait_for<T>(mut f: impl FnMut() -> Option<T>, timeout_ms: u64) -> Option<T> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Some(v) = f() {
            return Some(v);
        }
        if Instant::now() > deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn free_port() -> u16 {
    // Bind to :0 and let the OS pick a free port.
    let s = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    s.local_addr().unwrap().port()
}

// ── connect / disconnect ──────────────────────────────────────────────────────

#[test]
fn client_connects_to_server() {
    let port = free_port();
    let mut server = Server::bind(&format!("127.0.0.1:{port}")).unwrap();
    let _client = Client::connect(&format!("127.0.0.1:{port}")).unwrap();

    let connected = wait_for(
        || {
            for ev in server.poll() {
                if matches!(ev, ServerEvent::Connected(_)) {
                    return Some(true);
                }
            }
            None
        },
        2000,
    );
    assert!(
        connected.is_some(),
        "server did not receive Connected event"
    );
}

#[test]
fn client_receives_connected_event() {
    let port = free_port();
    let _server = Server::bind(&format!("127.0.0.1:{port}")).unwrap();
    let mut client = Client::connect(&format!("127.0.0.1:{port}")).unwrap();

    let connected = wait_for(
        || {
            for ev in client.poll() {
                if matches!(ev, ClientEvent::Connected) {
                    return Some(true);
                }
            }
            None
        },
        2000,
    );
    assert!(
        connected.is_some(),
        "client did not receive Connected event"
    );
}

#[test]
fn client_disconnect_notifies_server() {
    let port = free_port();
    let mut server = Server::bind(&format!("127.0.0.1:{port}")).unwrap();
    let client = Client::connect(&format!("127.0.0.1:{port}")).unwrap();

    // Wait for connection
    wait_for(
        || {
            for ev in server.poll() {
                if matches!(ev, ServerEvent::Connected(_)) {
                    return Some(());
                }
            }
            None
        },
        2000,
    )
    .expect("connection timeout");

    client.disconnect();

    let disconnected = wait_for(
        || {
            for ev in server.poll() {
                if matches!(ev, ServerEvent::Disconnected(_)) {
                    return Some(true);
                }
            }
            None
        },
        2000,
    );
    assert!(
        disconnected.is_some(),
        "server did not receive Disconnected event"
    );
}

// ── messaging ─────────────────────────────────────────────────────────────────

#[test]
fn client_sends_message_to_server() {
    let port = free_port();
    let mut server = Server::bind(&format!("127.0.0.1:{port}")).unwrap();
    let mut client = Client::connect(&format!("127.0.0.1:{port}")).unwrap();

    // Wait for connection
    wait_for(
        || {
            for ev in client.poll() {
                if matches!(ev, ClientEvent::Connected) {
                    return Some(());
                }
            }
            None
        },
        2000,
    )
    .expect("connect timeout");

    client.send(b"hello server").unwrap();

    let msg = wait_for(
        || {
            for ev in server.poll() {
                if let ServerEvent::Message(_, data) = ev {
                    return Some(data);
                }
            }
            None
        },
        2000,
    );
    assert_eq!(msg.unwrap(), b"hello server");
}

#[test]
fn server_sends_message_to_client() {
    let port = free_port();
    let mut server = Server::bind(&format!("127.0.0.1:{port}")).unwrap();
    let mut client = Client::connect(&format!("127.0.0.1:{port}")).unwrap();

    // Wait for server to see connection
    let client_id = wait_for(
        || {
            for ev in server.poll() {
                if let ServerEvent::Connected(id) = ev {
                    return Some(id);
                }
            }
            None
        },
        2000,
    )
    .expect("connect timeout");

    // Drain client Connected event
    wait_for(
        || {
            for ev in client.poll() {
                if matches!(ev, ClientEvent::Connected) {
                    return Some(());
                }
            }
            None
        },
        500,
    );

    server.send(client_id, b"hello client").unwrap();

    let msg = wait_for(
        || {
            for ev in client.poll() {
                if let ClientEvent::Message(data) = ev {
                    return Some(data);
                }
            }
            None
        },
        2000,
    );
    assert_eq!(msg.unwrap(), b"hello client");
}

#[test]
fn server_broadcasts_to_all_clients() {
    let port = free_port();
    let mut server = Server::bind(&format!("127.0.0.1:{port}")).unwrap();
    let mut c1 = Client::connect(&format!("127.0.0.1:{port}")).unwrap();
    let mut c2 = Client::connect(&format!("127.0.0.1:{port}")).unwrap();

    // Wait for both clients
    wait_for(
        || {
            server.poll();
            if server.client_count() >= 2 {
                Some(())
            } else {
                None
            }
        },
        2000,
    )
    .expect("clients did not connect");

    // Drain Connected events on clients
    std::thread::sleep(Duration::from_millis(50));
    c1.poll();
    c2.poll();

    server.broadcast(b"broadcast").unwrap();

    let r1 = wait_for(
        || {
            for ev in c1.poll() {
                if let ClientEvent::Message(d) = ev {
                    return Some(d);
                }
            }
            None
        },
        2000,
    );
    let r2 = wait_for(
        || {
            for ev in c2.poll() {
                if let ClientEvent::Message(d) = ev {
                    return Some(d);
                }
            }
            None
        },
        2000,
    );

    assert_eq!(r1.unwrap(), b"broadcast");
    assert_eq!(r2.unwrap(), b"broadcast");
}

// ── JSON ──────────────────────────────────────────────────────────────────────

#[test]
fn send_recv_json() {
    use nene::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Msg {
        x: f32,
        y: f32,
    }

    let port = free_port();
    let mut server = Server::bind(&format!("127.0.0.1:{port}")).unwrap();
    let mut client = Client::connect(&format!("127.0.0.1:{port}")).unwrap();

    wait_for(
        || {
            for ev in client.poll() {
                if matches!(ev, ClientEvent::Connected) {
                    return Some(());
                }
            }
            None
        },
        2000,
    )
    .expect("connect timeout");

    client.send_json(&Msg { x: 1.0, y: 2.0 }).unwrap();

    let msg = wait_for(
        || {
            for ev in server.poll() {
                if let Some((_, m)) = ev.into_json::<Msg>() {
                    return Some(m);
                }
            }
            None
        },
        2000,
    );
    assert_eq!(msg.unwrap(), Msg { x: 1.0, y: 2.0 });
}
