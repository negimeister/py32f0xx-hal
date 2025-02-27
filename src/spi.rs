//! API for the integrate SPI peripherals
//!
//! The spi bus acts as the master (generating the clock) and you need to handle the CS separately.
//!
//! The most significant bit is transmitted first & only 8-bit transfers are supported
//!
//! # Example
//! Echo incoming data in the next transfer
//! ``` no_run
//! use py32f0xx_hal as hal;
//!
//! use crate::hal::pac;
//! use crate::hal::prelude::*;
//! use crate::hal::spi::{Spi, Mode, Phase, Polarity};
//!
//! cortex_m::interrupt::free(|cs| {
//!     let mut p = pac::Peripherals::take().unwrap();
//!     let mut rcc = p.RCC.constrain().freeze(&mut p.FLASH);
//!
//!     let gpioa = p.GPIOA.split(&mut rcc);
//!
//!     // Configure pins for SPI
//!     let sck = gpioa.pa5.into_alternate_af0(cs);
//!     let miso = gpioa.pa6.into_alternate_af0(cs);
//!     let mosi = gpioa.pa7.into_alternate_af0(cs);
//!
//!     // Configure SPI with 1MHz rate
//!     let mut spi = Spi::spi1(p.SPI1, (sck, miso, mosi), Mode {
//!         polarity: Polarity::IdleHigh,
//!         phase: Phase::CaptureOnSecondTransition,
//!     }, 1.mhz(), &mut rcc);
//!
//!     let mut data = [0];
//!     loop {
//!         spi.transfer(&mut data).unwrap();
//!     }
//! });
//! ```

use core::marker::PhantomData;
use core::{ops::Deref, ptr};

pub use embedded_hal::spi::{Mode, Phase, Polarity};

// TODO Put this inside the macro
// Currently that causes a compiler panic
use crate::pac::SPI1;
#[cfg(any(
    feature = "py32f030",
))]
use crate::pac::SPI2;

use crate::gpio::*;

use crate::rcc::{Clocks, Rcc};

use crate::time::Hertz;

/// Typestate for 8-bit transfer size
pub struct EightBit;

/// Typestate for 16-bit transfer size
pub struct SixteenBit;

/// SPI error
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// Overrun occurred
    Overrun,
    /// Mode fault occurred
    ModeFault,
    /// CRC error
    Crc,
    /// Incomplete transfer
    IncompleteTransfer,
}

/// SPI abstraction
pub struct Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, WIDTH> {
    spi: SPI,
    pins: (SCKPIN, MISOPIN, MOSIPIN),
    _width: PhantomData<WIDTH>,
}

pub trait SckPin<SPI> {}
pub trait MisoPin<SPI> {}
pub trait MosiPin<SPI> {}

macro_rules! spi_pins {
    ($($SPI:ident => {
        sck => [$($sck:ty),+ $(,)*],
        miso => [$($miso:ty),+ $(,)*],
        mosi => [$($mosi:ty),+ $(,)*],
    })+) => {
        $(
            $(
                impl SckPin<crate::pac::$SPI> for $sck {}
            )+
            $(
                impl MisoPin<crate::pac::$SPI> for $miso {}
            )+
            $(
                impl MosiPin<crate::pac::$SPI> for $mosi {}
            )+
        )+
    }
}

#[cfg(any(
    feature = "lqfp32k1",
    feature = "lqfp32k2",
    feature = "qfn32k2",
))]
spi_pins! {
    SPI1 => {
        sck =>  [
            gpioa::PA1<Alternate<AF0>>,
            gpioa::PA2<Alternate<AF10>>,
            gpioa::PA5<Alternate<AF0>>,
            gpioa::PA9<Alternate<AF10>>,
            gpiob::PB3<Alternate<AF0>>,
        ],
        miso => [
            gpioa::PA0<Alternate<AF10>>,
            gpioa::PA3<Alternate<AF0>>,
            gpioa::PA6<Alternate<AF0>>,
            gpioa::PA7<Alternate<AF10>>,
            gpioa::PA11<Alternate<AF0>>,
            gpioa::PA13<Alternate<AF10>>,
            gpiob::PB4<Alternate<AF0>>,
        ],
        mosi => [
            gpioa::PA1<Alternate<AF10>>,
            gpioa::PA2<Alternate<AF0>>,
            gpioa::PA3<Alternate<AF10>>,
            gpioa::PA7<Alternate<AF0>>,
            gpioa::PA8<Alternate<AF10>>,
            gpioa::PA12<Alternate<AF0>>,
            gpiob::PB5<Alternate<AF0>>,
        ],
    }
}
#[cfg(any(
    feature = "ssop24e1",
    feature = "ssop24e2",
))]
spi_pins! {
    SPI1 => {
        sck =>  [
            gpioa::PA1<Alternate<AF0>>,
            gpioa::PA2<Alternate<AF10>>,
            gpioa::PA5<Alternate<AF0>>,
            gpiob::PB3<Alternate<AF0>>,
        ],
        miso => [
            gpioa::PA0<Alternate<AF10>>,
            gpioa::PA3<Alternate<AF0>>,
            gpioa::PA6<Alternate<AF0>>,
            gpioa::PA7<Alternate<AF10>>,
            gpioa::PA13<Alternate<AF10>>,
            gpiob::PB4<Alternate<AF0>>,
        ],
        mosi => [
            gpioa::PA1<Alternate<AF10>>,
            gpioa::PA2<Alternate<AF0>>,
            gpioa::PA3<Alternate<AF10>>,
            gpioa::PA7<Alternate<AF0>>,
            gpioa::PA12<Alternate<AF0>>,
            gpiob::PB5<Alternate<AF0>>,
        ],
    }
}

#[cfg(any(
    feature = "lqfp32k1",
    feature = "lqfp32k2",
    feature = "qfn32k2",
))]
spi_pins! {
    SPI2 => {
        sck =>  [
            gpioa::PA1<Alternate<AF0>>,
            gpiob::PB2<Alternate<AF1>>,
            gpiob::PB8<Alternate<AF1>>,
            gpiof::PF0<Alternate<AF3>>,
        ],
        miso => [
            gpioa::PA3<Alternate<AF0>>,
            gpioa::PA9<Alternate<AF0>>,
            gpiob::PB6<Alternate<AF3>>,
            gpiof::PF1<Alternate<AF3>>,
            gpiof::PF3<Alternate<AF3>>,
        ],
        mosi => [
            gpioa::PA4<Alternate<AF2>>,
            gpioa::PA10<Alternate<AF0>>,
            gpiob::PB7<Alternate<AF1>>,
            gpiof::PF2<Alternate<AF3>>,
        ],
    }
}
#[cfg(any(
    feature = "ssop24e1",
    feature = "ssop24e2",
))]
spi_pins! {
    SPI2 => {
        sck =>  [
            gpioa::PA1<Alternate<AF0>>,
            gpiob::PB2<Alternate<AF1>>,
            gpiob::PB8<Alternate<AF1>>,
            gpiof::PF0<Alternate<AF3>>,
        ],
        miso => [
            gpioa::PA3<Alternate<AF0>>,
            gpiof::PF1<Alternate<AF3>>,
        ],
        mosi => [
            gpioa::PA4<Alternate<AF2>>,
            gpiof::PF2<Alternate<AF3>>,
        ],
    }
}

macro_rules! spi {
    ($($SPI:ident: ($spi:ident, $spi_slave:ident, $spiXen:ident, $spiXrst:ident, $apbenr:ident, $apbrstr:ident),)+) => {
        $(
            impl<SCKPIN, MISOPIN, MOSIPIN> Spi<$SPI, SCKPIN, MISOPIN, MOSIPIN, EightBit> {
                /// Creates a new spi instance
                pub fn $spi<F>(
                    spi: $SPI,
                    pins: (SCKPIN, MISOPIN, MOSIPIN),
                    mode: Mode,
                    speed: F,
                    rcc: &mut Rcc,
                ) -> Self
                where
                    SCKPIN: SckPin<$SPI>,
                    MISOPIN: MisoPin<$SPI>,
                    MOSIPIN: MosiPin<$SPI>,
                    F: Into<Hertz>,
                {   
                    /* Enable clock for SPI */
                    rcc.regs.$apbenr.modify(|_, w| w.$spiXen().set_bit());
                    /* Reset SPI */
                    rcc.regs.$apbrstr.modify(|_, w| w.$spiXrst().set_bit());
                    rcc.regs.$apbrstr.modify(|_, w| w.$spiXrst().clear_bit());
                    Spi::<$SPI, SCKPIN, MISOPIN, MOSIPIN, EightBit> { spi, pins, _width: PhantomData }.spi_init(mode, speed, rcc.clocks).into_8bit_width().enable()
                }

                pub fn $spi_slave(
                    spi: $SPI,
                    pins: (SCKPIN, MISOPIN, MOSIPIN),
                    mode: Mode,
                    rcc: &mut Rcc,
                ) -> Self
                where
                    SCKPIN: SckPin<$SPI>,
                    MISOPIN: MisoPin<$SPI>,
                    MOSIPIN: MosiPin<$SPI>,
                {   
                    /* Enable clock for SPI */
                    rcc.regs.$apbenr.modify(|_, w| w.$spiXen().set_bit());
                    /* Reset SPI */
                    rcc.regs.$apbrstr.modify(|_, w| w.$spiXrst().set_bit());
                    rcc.regs.$apbrstr.modify(|_, w| w.$spiXrst().clear_bit());
                        Spi::<$SPI, SCKPIN, MISOPIN, MOSIPIN, EightBit> { spi, pins, _width: PhantomData }.spi_init_slave(mode).into_8bit_width().enable()
                }
                
                pub fn reset(&mut self, rcc: &mut Rcc){
                    let cr1 = self.spi.cr1.read();
                    let cr2 = self.spi.cr2.read();
                    rcc.regs.$apbrstr.modify(|_, w| w.$spiXrst().set_bit());
                    rcc.regs.$apbrstr.modify(|_, w| w.$spiXrst().clear_bit());
                    self.spi.cr2.write(|w| unsafe { w.bits(cr2.bits()) });
                    self.spi.cr1.write(|w| unsafe { w.bits(cr1.bits()) });
                }
            }
            
        )+
    }
}

spi! {
    SPI1: (spi1, spi1_slave, spi1en, spi1rst, apbenr2, apbrstr2),
}
#[cfg(any(
    feature = "py32f030",
    feature = "py32f003",
))]
spi! {
    SPI2: (spi2, spi2_slave, spi2en, spi2rst, apbenr1, apbrstr1),
}

// It's s needed for the impls, but rustc doesn't recognize that
#[allow(dead_code)]
type SpiRegisterBlock = crate::pac::spi1::RegisterBlock;

impl<SPI, SCKPIN, MISOPIN, MOSIPIN, WIDTH> Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, WIDTH>
where
    SPI: Deref<Target = SpiRegisterBlock>,
{
    fn spi_init<F>(self, mode: Mode, speed: F, clocks: Clocks) -> Self
    where
        F: Into<Hertz>,
    {
        /* Make sure the SPI unit is disabled so we can configure it */
        self.spi.cr1.modify(|_, w| w.spe().clear_bit());

        let br = match clocks.pclk().0 / speed.into().0 {
            0 => unreachable!(),
            1..=2 => 0b000,
            3..=5 => 0b001,
            6..=11 => 0b010,
            12..=23 => 0b011,
            24..=47 => 0b100,
            48..=95 => 0b101,
            96..=191 => 0b110,
            _ => 0b111,
        };

        // mstr: master configuration
        // lsbfirst: MSB first
        // ssm: enable software slave management (NSS pin free for other uses)
        // ssi: set nss high = master mode
        // dff: 8 bit frames
        // bidimode: 2-line unidirectional
        // spe: enable the SPI bus
        self.spi.cr1.write(|w| {
            w.cpha()
                .bit(mode.phase == Phase::CaptureOnSecondTransition)
                .cpol()
                .bit(mode.polarity == Polarity::IdleHigh)
                .mstr()
                .set_bit()
                .br()
                .bits(br)
                .lsbfirst()
                .clear_bit()
                .ssm()
                .set_bit()
                .ssi()
                .set_bit()
                .rxonly()
                .clear_bit()
                .bidimode()
                .clear_bit()
        });

        self
    }
    fn spi_init_slave(self, mode: Mode) -> Self
        {
        /* Make sure the SPI unit is disabled so we can configure it */
        self.spi.cr1.modify(|_, w| w.spe().clear_bit());


        // mstr: slave configuration
        // lsbfirst: MSB first
        // ssm: enable hardware chip select
        // dff: 8 bit frames
        // bidimode: 2-line unidirectional
        // spe: enable the SPI bus
        self.spi.cr1.write(|w| {
            w.cpha()
                .bit(mode.phase == Phase::CaptureOnSecondTransition)
                .cpol()
                .bit(mode.polarity == Polarity::IdleHigh)
                .mstr()
                .clear_bit()
                .lsbfirst()
                .clear_bit()
                .ssm()
                .clear_bit()
                .ssi()
                .clear_bit()
                .rxonly()
                .clear_bit()
                .bidimode()
                .clear_bit()
        });

        self
    }


    pub fn enable(self) -> Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, WIDTH> {
        self.spi.cr1.modify(|_, w| w.spe().set_bit());
        self
    }

    pub fn into_8bit_width(self) -> Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, EightBit> {
        // FRXTH: 8-bit threshold on RX FIFO
        // DS: 8-bit data size
        // SSOE: cleared to disable SS output
        self.spi
            .cr2
            .write(|w| w.frxth().set_bit().ds().eight_bit().ssoe().clear_bit());

        Spi {
            spi: self.spi,
            pins: self.pins,
            _width: PhantomData,
        }
    }

    pub fn into_16bit_width(self) -> Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, SixteenBit> {
        // FRXTH: 16-bit threshold on RX FIFO
        // DS: 8-bit data size
        // SSOE: cleared to disable SS output
        self.spi
            .cr2
            .write(|w| w.frxth().set_bit().ds().sixteen_bit().ssoe().clear_bit());

        Spi {
            spi: self.spi,
            pins: self.pins,
            _width: PhantomData,
        }
    }

    fn set_send_only(&mut self) {
        self.spi
            .cr1
            .modify(|_, w| w.bidimode().set_bit().bidioe().set_bit());
    }

    fn set_bidi(&mut self) {
        self.spi
            .cr1
            .modify(|_, w| w.bidimode().clear_bit().bidioe().clear_bit());
    }

    fn check_read(&mut self) -> nb::Result<(), Error> {
        let sr = self.spi.sr.read();

        Err(if sr.ovr().bit_is_set() {
            nb::Error::Other(Error::Overrun)
        } else if sr.modf().bit_is_set() {
            nb::Error::Other(Error::ModeFault)
        } else if sr.rxne().bit_is_set() {
            return Ok(());
        } else {
            nb::Error::WouldBlock
        })
    }

    fn send_buffer_size(&mut self) -> u8 {
        match self.spi.sr.read().ftlvl().bits() {
            // FIFO empty
            0 => 4,
            // FIFO 1/4 full
            1 => 3,
            // FIFO 1/2 full
            2 => 2,
            // FIFO full
            _ => 0,
        }
    }

    fn check_send(&mut self) -> nb::Result<(), Error> {
        let sr = self.spi.sr.read();

        Err(if sr.ovr().bit_is_set() {
            nb::Error::Other(Error::Overrun)
        } else if sr.modf().bit_is_set() {
            nb::Error::Other(Error::ModeFault)
        } else if sr.txe().bit_is_set() && sr.bsy().bit_is_clear() {
            return Ok(());
        } else {
            nb::Error::WouldBlock
        })
    }

    fn read_u8(&mut self) -> u8 {
        // NOTE(read_volatile) read only 1 byte (the svd2rust API only allows reading a half-word)
        unsafe { ptr::read_volatile(&self.spi.dr as *const _ as *const u8) }
    }

    fn send_u8(&mut self, byte: u8) {
        // NOTE(write_volatile) see note above
        unsafe { ptr::write_volatile(self.spi.dr.as_ptr() as *mut u8, byte) }
    }

    fn read_u16(&mut self) -> u16 {
        // NOTE(read_volatile) read only 2 bytes (the svd2rust API only allows reading a half-word)
        unsafe { ptr::read_volatile(&self.spi.dr as *const _ as *const u16) }
    }

    fn send_u16(&mut self, byte: u16) {
        // NOTE(write_volatile) see note above
        unsafe { ptr::write_volatile(self.spi.dr.as_ptr() as *mut u16, byte) }
    }

    pub fn release(self) -> (SPI, (SCKPIN, MISOPIN, MOSIPIN)) {
        (self.spi, self.pins)
    }
}

impl<SPI, SCKPIN, MISOPIN, MOSIPIN> ::embedded_hal::blocking::spi::Transfer<u8>
    for Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, EightBit>
where
    SPI: Deref<Target = SpiRegisterBlock>,
{
    type Error = Error;
    
    fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'w [u8], Self::Error> {
        self.set_bidi(); // Ensure we're in the correct mode for bidirectional transfer
    
        let mut read_pos = 0; // Keep track of the position we're reading from
        let mut write_pos = 0; // Keep track of the position we're writing to
    
        while read_pos < words.len() {
            // Fill the transmit FIFO as much as possible
            while write_pos < words.len() && self.check_send().is_ok() {
                self.send_u8(words[write_pos]);
                write_pos += 1;
            }
    
            // Read from the receive FIFO whenever possible
            while self.check_read().is_ok() && read_pos < write_pos {
                words[read_pos] = self.read_u8();
                read_pos += 1;
            }
        }
    
        // After all writes are done, there might still be data left in the receive FIFO.
        // Drain the receive FIFO.
        while read_pos < words.len() {
            if self.check_read().is_ok() {
                words[read_pos] = self.read_u8();
                read_pos += 1;
            } else {
                // If there's nothing to read but we haven't received all data, it's an error.
                return Err(Error::IncompleteTransfer);
            }
        }
    
        // Check for overrun after the transfer is complete
        if self.spi.sr.read().ovr().bit_is_set() {
            Err(Error::Overrun)
        } else {
            Ok(words)
        }
    }
       
}

impl<SPI, SCKPIN, MISOPIN, MOSIPIN> ::embedded_hal::blocking::spi::Write<u8>
    for Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, EightBit>
where
    SPI: Deref<Target = SpiRegisterBlock>,
{
    type Error = Error;

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        let mut bufcap: u8 = 0;

        // We only want to send, so we don't need to worry about the receive buffer overflowing
        self.set_send_only();

        // Make sure we don't continue with an error condition
        nb::block!(self.check_send())?;

        // We have a 32 bit buffer to work with, so let's fill it before checking the status
        for word in words {
            // Loop as long as our send buffer is full
            while bufcap == 0 {
                bufcap = self.send_buffer_size();
            }

            self.send_u8(*word);
            bufcap -= 1;
        }

        // Do one last status register check before continuing
        nb::block!(self.check_send()).ok();
        Ok(())
    }
}

impl<SPI, SCKPIN, MISOPIN, MOSIPIN> ::embedded_hal::blocking::spi::Transfer<u16>
    for Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, SixteenBit>
where
    SPI: Deref<Target = SpiRegisterBlock>,
{
    type Error = Error;

    fn transfer<'w>(&mut self, words: &'w mut [u16]) -> Result<&'w [u16], Self::Error> {
        // We want to transfer bidirectionally, make sure we're in the correct mode
        self.set_bidi();

        for word in words.iter_mut() {
            nb::block!(self.check_send())?;
            self.send_u16(*word);
            nb::block!(self.check_read())?;
            *word = self.read_u16();
        }

        Ok(words)
    }
}

impl<SPI, SCKPIN, MISOPIN, MOSIPIN> ::embedded_hal::blocking::spi::Write<u16>
    for Spi<SPI, SCKPIN, MISOPIN, MOSIPIN, SixteenBit>
where
    SPI: Deref<Target = SpiRegisterBlock>,
{
    type Error = Error;

    fn write(&mut self, words: &[u16]) -> Result<(), Self::Error> {
        // We only want to send, so we don't need to worry about the receive buffer overflowing
        self.set_send_only();

        for word in words {
            nb::block!(self.check_send())?;
            self.send_u16(*word);
        }

        // Do one last status register check before continuing
        nb::block!(self.check_send()).ok();
        Ok(())
    }
}
