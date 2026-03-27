//! Multiplayer demo — relay server.
//!
//! Run this first, then run two instances of `multiplayer_client`.
//!
//!   cargo run --example multiplayer_server
//!
//! The server relays each client's position to all other clients.

use nene::net::{Server, ServerEvent};
use std::collections::HashMap;
use std::time::Duration;

#[nene::net_message]
#[derive(Debug)]
struct PlayerState {
    id: u32,
    x: f32,
    y: f32,
}

fn main() {
    let mut server = Server::bind("0.0.0.0:7777").expect("bind failed");
    println!("Multiplayer server listening on :7777");

    // Latest known state per client
    let mut states: HashMap<u32, PlayerState> = HashMap::new();
    let mut last = std::time::Instant::now();

    loop {
        let now = std::time::Instant::now();
        let dt = now.duration_since(last).as_secs_f32();
        last = now;

        for event in server.poll() {
            match &event {
                ServerEvent::Connected(id) => {
                    println!(
                        "Player {} connected  (total: {})",
                        id,
                        server.client_count()
                    );
                    for state in states.values() {
                        let _ = server.send_json(*id, state);
                    }
                }
                ServerEvent::Disconnected(id) => {
                    println!("Player {} disconnected", id);
                    states.remove(&id.0);
                    let gone = PlayerState {
                        id: id.0,
                        x: f32::NAN,
                        y: f32::NAN,
                    };
                    let _ = server.broadcast_json(&gone);
                }
                ServerEvent::Message(id, _) => {
                    if let Some((_, mut state)) = event.into_json::<PlayerState>() {
                        state.id = id.0;
                        states.insert(id.0, state.clone());
                        for &other in server.client_ids().iter().filter(|&&c| c != *id) {
                            let _ = server.send_json(other, &state);
                        }
                    }
                }
            }
        }

        server.update(dt);
        std::thread::sleep(Duration::from_millis(16));
    }
}
