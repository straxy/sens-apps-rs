mod i2csens;
mod mmsens;
mod sensor;
mod spisens;

use i2csens::I2CSens;
use mmsens::MMSens;
use sensor::Sensor;
use spisens::SPISens;

use async_channel::bounded;
use chrono::Utc;
use smol_macros::Executor;

pub(crate) enum TaskId {
    Mm = 0x01,
    I2C = 0x02,
    Spi = 0x04,
}

const ALL_TASKS: u8 = (TaskId::Mm as u8) | (TaskId::I2C as u8) | (TaskId::Spi as u8);

pub(crate) enum UpdateMessage {
    MemoryMapped {
        timestamp: chrono::DateTime<Utc>,
        value: u32,
    },
    I2C {
        timestamp: chrono::DateTime<Utc>,
        value: f32,
    },
    Spi {
        timestamp: chrono::DateTime<Utc>,
        value: f32,
    },
    TaskDone {
        taskid: TaskId,
    },
}

pub(crate) struct Sens {
    receiver: async_channel::Receiver<UpdateMessage>,
    tasks: u8,
}

impl Sens {
    pub(crate) async fn spawn(
        ex: &Executor<'_>,
        ctrl_c_receiver: async_broadcast::Receiver<()>,
    ) -> anyhow::Result<()> {
        let (sender, receiver) = bounded(10);
        let actor = Self {
            receiver,
            tasks: ALL_TASKS,
        };

        let runner = ex.spawn(actor.run());
        MMSens::spawn(ex, sender.clone(), ctrl_c_receiver.clone()).await?;
        I2CSens::spawn(ex, sender.clone(), ctrl_c_receiver.clone()).await?;
        SPISens::spawn(ex, sender.clone(), ctrl_c_receiver.clone()).await?;
        runner.await;
        Ok(())
    }

    pub(crate) async fn run(mut self) {
        loop {
            match self.receiver.recv().await.unwrap() {
                UpdateMessage::MemoryMapped { timestamp, value } => {
                    println!("[{timestamp:?}] MMSens : {value}")
                }
                UpdateMessage::I2C { timestamp, value } => {
                    println!("[{timestamp:?}] I2CSens: {value}")
                }
                UpdateMessage::Spi { timestamp, value } => {
                    println!("[{timestamp:?}] SPISens: {value}")
                }
                UpdateMessage::TaskDone { taskid } => {
                    self.tasks &= !(taskid as u8);
                    if self.tasks == 0 {
                        println!("All tasks are done, exit");
                        break;
                    }
                }
            }
        }
    }
}
