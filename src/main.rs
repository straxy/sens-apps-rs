//! Uses the `signal-hook` crate to catch the Ctrl-C signal.
//!
//! Run with:
//!
//! ```
//! cargo run --example unix-signal
//! ```
use signal_hook::{consts, low_level};
use smol::{Async, prelude::*};
use smol_macros::{Executor, main};
use std::os::unix::{io::AsRawFd, net::UnixStream};

main! {
    async fn main(ex: &Executor<'_>) {
        ex.spawn(async {
            // Create a Unix stream that receives a byte on each signal occurrence.
            let (a, mut b) = Async::<UnixStream>::pair().unwrap();
            // Async isn't IntoRawFd, but it is AsRawFd, so let's pass the raw fd directly.
            low_level::pipe::register_raw(consts::SIGINT, a.as_raw_fd()).unwrap();
            println!("Waiting for Ctrl-C...");

            // Receive a byte that indicates the Ctrl-C signal occurred.
            b.read_exact(&mut [0]).await.unwrap();

            println!("Done!");
        }).await;
    }
}
