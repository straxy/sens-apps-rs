use smol_macros::{main, Executor};

pub mod sens;

use sens::Sens;

main! {
    async fn main(ex: &Executor<'_>) -> anyhow::Result<()> {
        // Set a handler that sends a message through a channel.
        let (ctrl_c_sender, ctrl_c_receiver) = async_broadcast::broadcast(10);
        let handle = move || {
            ctrl_c_sender.try_broadcast(()).ok();
        };
        ctrlc::set_handler(handle).unwrap();

        Sens::spawn(ex, ctrl_c_receiver).await
    }
}
