use super::UpdateMessage;
use async_broadcast::Receiver;
use async_channel::Sender;
use smol_macros::Executor;

pub(super) trait Sensor {
    async fn spawn(
        ex: &Executor<'_>,
        sender: Sender<UpdateMessage>,
        ctrl_c_receiver: Receiver<()>,
    ) -> anyhow::Result<()>;
    async fn init(&mut self) -> anyhow::Result<()>;
    async fn deinit(&mut self);
    async fn run(self);
}
