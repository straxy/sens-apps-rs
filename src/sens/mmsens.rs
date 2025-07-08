use super::TaskId;
use super::{sensor::Sensor, UpdateMessage};
use async_broadcast::Receiver;
use async_channel::Sender;
use async_fs::File;
use polling::{Event, Events, Poller};
use smol_macros::Executor;
use std::path::Path;

const MMSENS_PATH: &str = "/sys/class/mmsens/mmsens0/";
const ENABLE_ATTR: &str = "enable";
const ENABLE_INTERRUPT_ATTR: &str = "enable_interrupt";
const DATA_ATTR: &str = "data";
const INITVAL_ATTR: &str = "initval";
const FREQUENCY_ATTR: &str = "frequency";
const INTERRUPT_ATTR: &str = "interrupt";
const MMSENS_EV: usize = 1;
const ENABLE: u32 = 1;
const DISABLE: u32 = 0;

pub(super) struct MMSens {
    sender: Sender<UpdateMessage>,
    ctrl_c_receiver: Receiver<()>,
}

impl Sensor for MMSens {
    async fn spawn(
        ex: &Executor<'_>,
        sender: Sender<UpdateMessage>,
        ctrl_c_receiver: Receiver<()>,
    ) -> anyhow::Result<()> {
        let mut mmsens = Self {
            sender,
            ctrl_c_receiver,
        };
        mmsens.init().await?;
        ex.spawn(mmsens.run()).detach();
        Ok(())
    }

    async fn init(&mut self) -> anyhow::Result<()> {
        // Write initial value
        MMSens::write_attr_u32(INITVAL_ATTR, 0).await?;

        // Enable device
        MMSens::write_attr_u32(ENABLE_ATTR, ENABLE).await?;

        // Configure frequency
        MMSens::write_attr_str(FREQUENCY_ATTR, "normal").await?;

        // Enable interrupt
        MMSens::write_attr_u32(ENABLE_INTERRUPT_ATTR, ENABLE).await?;

        Ok(())
    }

    async fn deinit(&mut self) {
        // Disable interrupt
        let _ = MMSens::write_attr_u32(ENABLE_INTERRUPT_ATTR, DISABLE).await;
        // Disable device
        let _ = MMSens::write_attr_u32(ENABLE_ATTR, DISABLE).await;
    }

    async fn run(mut self) {
        let poller = Poller::new().unwrap();
        let file = File::open(Path::new(MMSENS_PATH).join(INTERRUPT_ATTR))
            .await
            .expect("Could not open interrupt file");
        unsafe {
            poller
                .add_with_mode(&file, Event::readable(MMSENS_EV), polling::PollMode::Edge)
                .expect("Could not add file to poller")
        };
        let mut events = Events::new();
        loop {
            events.clear();
            poller
                .wait(&mut events, Some(std::time::Duration::from_millis(500)))
                .expect("Poll failed!");

            let mut event = false;
            for ev in events.iter() {
                if ev.key == MMSENS_EV {
                    event = true;
                }
            }
            if event {
                let value = MMSens::read_attr_u32(DATA_ATTR).await.unwrap_or_default();
                self.sender
                    .send(UpdateMessage::MemoryMapped {
                        timestamp: chrono::Utc::now(),
                        value,
                    })
                    .await
                    .expect("Failed to send data");
            }
            if self.ctrl_c_receiver.try_recv().is_ok() {
                println!("MMSens: Ctrl+C received");
                self.deinit().await;
                let _ = self
                    .sender
                    .send(UpdateMessage::TaskDone { taskid: TaskId::Mm })
                    .await;
                break;
            }
        }
    }
}

impl MMSens {
    async fn read_attr_str(attr: &str) -> anyhow::Result<String> {
        let attr_path = Path::new(MMSENS_PATH).join(attr);
        async_fs::read(attr_path)
            .await
            .map(|bytes| String::from_utf8(bytes).unwrap_or_default())
            .map_err(|e| e.into())
    }

    async fn read_attr_u32(attr: &str) -> anyhow::Result<u32> {
        MMSens::read_attr_str(attr)
            .await
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .map_err(|e| e.into())
    }

    async fn write_attr_str(attr: &str, value: &str) -> anyhow::Result<()> {
        let attr_path = Path::new(MMSENS_PATH).join(attr);
        async_fs::write(attr_path, value.as_bytes())
            .await
            .map_err(|e| e.into())
    }

    async fn write_attr_u32(attr: &str, value: u32) -> anyhow::Result<()> {
        let value_u32: String = value.to_string();
        MMSens::write_attr_str(attr, value_u32.as_str()).await
    }
}
