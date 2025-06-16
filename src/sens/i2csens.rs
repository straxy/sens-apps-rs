use super::TaskId;
use super::{sensor::Sensor, UpdateMessage};
use async_broadcast::Receiver;
use async_channel::Sender;
use async_io::Timer;
use i2cdev::core::{I2CMessage, I2CTransfer};
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CMessage};
use smol::stream::StreamExt;
use smol_macros::Executor;
use std::time::Duration;

const I2CSENS_ADDR: u16 = 0x36;
const CTRL_ENABLE: u8 = 0x01;
const I2C_PATH: &str = "/dev/i2c-1";

enum I2CReg {
    #[allow(unused)]
    Id,
    Ctrl,
    Temperature,
}

pub(super) struct I2CSens {
    sender: Sender<UpdateMessage>,
    ctrl_c_receiver: Receiver<()>,
    i2c_device: LinuxI2CDevice,
}

impl Sensor for I2CSens {
    async fn spawn(
        ex: &Executor<'_>,
        sender: Sender<UpdateMessage>,
        ctrl_c_receiver: Receiver<()>,
    ) -> anyhow::Result<()> {
        let i2c_device = LinuxI2CDevice::new(I2C_PATH, I2CSENS_ADDR)?;
        let mut i2csens = Self {
            sender,
            ctrl_c_receiver,
            i2c_device,
        };
        i2csens.init().await?;
        ex.spawn(i2csens.run()).detach();
        Ok(())
    }

    async fn init(&mut self) -> anyhow::Result<()> {
        self.write_reg(I2CReg::Ctrl, CTRL_ENABLE).await
    }

    async fn deinit(&mut self) {
        let _ = self.write_reg(I2CReg::Ctrl, 0).await;
    }

    async fn run(mut self) {
        let mut timeout = Timer::interval(Duration::from_secs(1));
        loop {
            if timeout.next().await.is_some() {
                // Read and send
                if let Ok(value) = self.read_reg(I2CReg::Temperature).await {
                    let _ = self
                        .sender
                        .send(UpdateMessage::I2C {
                            timestamp: chrono::Utc::now(),
                            value: value as f32 / 2f32,
                        })
                        .await;
                }
            }
            if self.ctrl_c_receiver.try_recv().is_ok() {
                println!("I2C: Ctrl+C received");
                self.deinit().await;
                let _ = self
                    .sender
                    .send(UpdateMessage::TaskDone {
                        taskid: TaskId::I2C,
                    })
                    .await;
                break;
            }
        }
    }
}

impl I2CSens {
    async fn read_reg(&mut self, reg_nr: I2CReg) -> anyhow::Result<u8> {
        let mut read_data = [0; 1];
        let reg = [reg_nr as u8];
        let mut msgs = [
            LinuxI2CMessage::write(&reg),
            LinuxI2CMessage::read(&mut read_data),
        ];
        self.i2c_device
            .transfer(&mut msgs)
            .map(|_| read_data[0])
            .map_err(|e| e.into())
    }

    async fn write_reg(&mut self, reg_nr: I2CReg, value: u8) -> anyhow::Result<()> {
        let reg = [reg_nr as u8, value];
        let mut msgs = [LinuxI2CMessage::write(&reg)];
        self.i2c_device
            .transfer(&mut msgs)
            .map(|_| ())
            .map_err(|e| e.into())
    }
}
