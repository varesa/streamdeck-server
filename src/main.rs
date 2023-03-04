use futures::{StreamExt, SinkExt};
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::lock::Mutex;
use futures::stream::{SplitStream, SplitSink};
use streamdeck::StreamDeck;
use tokio::task::yield_now;
use warp::Filter;
use warp::ws::Message;
use warp::ws::WebSocket;
use lazy_static::lazy_static;


async fn do_nothing() {}

struct Hub {
    ws_clients: Mutex<Vec<Sender<Vec<u8>>>>,
}

impl Hub {
    fn new() -> Self {
        Self {
            ws_clients: Mutex::new(Vec::new()),
        }
    }

    async fn register_client(&self) -> Receiver<Vec<u8>> {
        let (tx, rx) = channel(10); 
        self.ws_clients.lock().await.push(tx);
        rx
    }

    async fn worker(&self) {
        let mut deck = StreamDeck::connect(0x0fd9, 0x006d, None).expect("Failed to open StreamDeck");
        deck.set_blocking(false)
            .expect("Failed to set non-blocking");
        loop {
            let buttons = deck.read_buttons(None); //.expect("Failed to get buttons");
            if let Ok(buttons) = buttons {
                println!("Got event from StreamDeck");
                let mut cleanup_needed = false;
                for client in self.ws_clients.lock().await.iter_mut() {
                    if let Err(e) = client.send(buttons.clone()).await {
                        dbg!(e);
                        cleanup_needed = true;
                    }
                }
                if cleanup_needed {
                    self.ws_clients.lock().await.retain(|elem| !elem.is_closed());
                }
                dbg!(buttons);
            }
            yield_now().await;
        }
    }
}

lazy_static! {
    static ref HUB: Hub = Hub::new();
}

async fn client_handler(mut tx: SplitSink<WebSocket, Message>, _rx: SplitStream<WebSocket>) {
    println!("Client handler started");
    let mut streamdeck_channel = HUB.register_client().await;
    while let Some(event) = streamdeck_channel.next().await {
        println!("Got event from hub");
        if let Err(e) = tx.send(Message::binary(event)).await {
            dbg!(e);
            streamdeck_channel.close();
            break;
        }
    }
}

#[tokio::main]
async fn main() {
    tokio::spawn(HUB.worker());
    let wsroute = warp::path("ws").and(warp::ws()).map(|ws: warp::ws::Ws| {
        ws.on_upgrade(|websocket| {
            println!("Got a websocket client");
            let (tx, rx) = websocket.split();
            tokio::spawn(client_handler(tx, rx));
            do_nothing()
        })
    });

    warp::serve(warp::get().and(wsroute))
        .run(([0, 0, 0, 0], 8085))
        .await;
}
