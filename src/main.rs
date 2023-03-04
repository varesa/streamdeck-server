use futures::FutureExt;
use futures::StreamExt;
use streamdeck::{Colour, StreamDeck};
use warp::Filter;

fn test_deck() {
    let mut deck = StreamDeck::connect(0x0fd9, 0x006d, None).expect("Failed to open StreamDeck");
    deck.set_button_rgb(0, &Colour { r: 255, g: 0, b: 0 })
        .expect("Failed to set colour");
    deck.set_button_rgb(
        1,
        &Colour {
            r: 0,
            g: 0xff,
            b: 0,
        },
    )
    .expect("Failed to set colour");
    deck.set_button_rgb(
        2,
        &Colour {
            r: 0,
            g: 0,
            b: 0xff,
        },
    )
    .expect("Failed to set colour");
    deck.set_blocking(false)
        .expect("Failed to set non-blocking");
    loop {
        let buttons = deck.read_buttons(None); //.expect("Failed to get buttons");
        if let Ok(buttons) = buttons {
            dbg!(buttons);
        }
    }
}

#[tokio::main]
async fn main() {
    let wsroute = warp::path("ws").and(warp::ws()).map(|ws: warp::ws::Ws| {
        ws.on_upgrade(|websocket| {
            let (tx, rx) = websocket.split();
            rx.forward(tx).map(|result| ())
        })
    });

    warp::serve(warp::get().and(wsroute))
        .run(([0, 0, 0, 0], 8085))
        .await;
}
