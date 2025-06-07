//! Uses the `signal-hook` crate to catch the Ctrl-C signal.
//!
//! Run with:
//!
//! ```
//! cargo run --example unix-signal
//! ```
use async_channel::{bounded, Receiver, Sender};
use smol::{io, Async};
use smol::{prelude::*, stream};
use smol_macros::{main, Executor};
use std::time::{Duration, Instant};
use timerfd::{SetTimeFlags, TimerFd, TimerState};

enum STest {
    TEST1,
    TEST2,
}

main! {
    async fn main(ex: &Executor<'_>) {
        let (sender, receiver) = bounded(10);

        let t1 = ex.spawn(run_send(sender));

        let t2 = ex.spawn(run_receive(receiver));

        let handles: Vec<smol::Task<()>> = vec![t1, t2];

        for handle in handles {
            handle.await;
        }
    }
}

/// Sleeps using an OS timer.
async fn sleep(dur: Duration) -> io::Result<()> {
    // Create an OS timer.
    let mut timer = TimerFd::new()?;
    timer.set_state(TimerState::Oneshot(dur), SetTimeFlags::Default);

    // When the OS timer fires, a 64-bit integer can be read from it.
    Async::new(timer)?
        .read_with(|t| rustix::io::read(t, &mut [0u8; 8]).map_err(io::Error::from))
        .await?;
    Ok(())
}

async fn run_receive(receiver: Receiver<STest>) {
    let start = Instant::now();
    println!("Sleeping...");
    receiver.recv().await;

    println!("Received after {:?}", start.elapsed());
}
async fn run_send(sender: Sender<STest>) {
    let start = Instant::now();
    println!("Sleeping...");

    // Sleep for a second using an OS timer.
    sleep(Duration::from_secs(1)).await.unwrap();

    println!("Woke up after {:?}", start.elapsed());
    sender.send(STest::TEST1).await;
}
