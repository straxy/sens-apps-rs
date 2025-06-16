use super::TaskId;
use super::{sensor::Sensor, UpdateMessage};
use async_broadcast::Receiver;
use async_channel::Sender;
use async_io::Timer;
use smol::stream::StreamExt;
use smol_macros::Executor;
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};
use std::time::Duration;

enum SPIReg {
    #[allow(unused)]
    Id,
    Ctrl,
    Temperature,
}

// Enable value
const CTRL_ENABLE: u8 = 0x01;
// Command operation flag: 0 - read, 1 - write
const CMD_OP_WRITE: u8 = 0x80;
// Command register index shift
const CMD_REG_SHIFT: u8 = 4;
const SPI_PATH: &str = "/dev/spidev0.0";

pub(super) struct SPISens {
    sender: Sender<UpdateMessage>,
    ctrl_c_receiver: Receiver<()>,
    spi_dev: Spidev,
}

impl Sensor for SPISens {
    async fn spawn(
        ex: &Executor<'_>,
        sender: Sender<UpdateMessage>,
        ctrl_c_receiver: Receiver<()>,
    ) -> anyhow::Result<()> {
        let mut spisens = Self {
            sender,
            ctrl_c_receiver,
            spi_dev: SPISens::create_spi().unwrap(),
        };
        spisens.init().await?;
        ex.spawn(spisens.run()).detach();
        Ok(())
    }

    async fn init(&mut self) -> anyhow::Result<()> {
        self.write_reg(SPIReg::Ctrl, CTRL_ENABLE).await
    }

    async fn deinit(&mut self) {
        let _ = self.write_reg(SPIReg::Ctrl, 0).await;
    }

    async fn run(mut self) {
        let mut timeout = Timer::interval(Duration::from_secs(1));
        loop {
            if timeout.next().await.is_some() {
                if let Ok(value) = self.read_reg(SPIReg::Temperature).await {
                    let _ = self
                        .sender
                        .send(UpdateMessage::Spi {
                            timestamp: chrono::Utc::now(),
                            value: value as f32 / 2f32,
                        })
                        .await;
                }
            }
            if self.ctrl_c_receiver.try_recv().is_ok() {
                println!("SPI: Ctrl+C received");
                self.deinit().await;
                let _ = self
                    .sender
                    .send(UpdateMessage::TaskDone {
                        taskid: TaskId::Spi,
                    })
                    .await;
                break;
            }
        }
    }
}

impl SPISens {
    fn create_spi() -> anyhow::Result<Spidev> {
        let mut spi = Spidev::open(SPI_PATH)?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(20_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&options)?;
        Ok(spi)
    }

    async fn read_reg(&mut self, reg_id: SPIReg) -> anyhow::Result<u8> {
        let cmd_byte = (reg_id as u8) << CMD_REG_SHIFT;
        let tx_buf = [cmd_byte, 0u8];
        let mut rx_buf = [0; 2];
        {
            let mut transfer = SpidevTransfer::read_write(&tx_buf, &mut rx_buf);
            self.spi_dev.transfer(&mut transfer)?;
        }
        Ok(rx_buf[1])
    }

    async fn write_reg(&mut self, reg_id: SPIReg, value: u8) -> anyhow::Result<()> {
        let cmd_byte = ((reg_id as u8) << CMD_REG_SHIFT) | CMD_OP_WRITE;
        let tx_buf = [cmd_byte, value];
        let mut rx_buf = [0; 2];
        {
            let mut transfer = SpidevTransfer::read_write(&tx_buf, &mut rx_buf);
            self.spi_dev.transfer(&mut transfer)?;
        }
        Ok(())
    }
}
