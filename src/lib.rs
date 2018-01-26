//  K8055.rs is a library for controlling the Vellemann K8055 USB IO card from rust.
//  Copyright (C) 2014 Falco Hirschenberger <falco.hirschenberger@gmail.com>
//
//  This program is free software; you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation; either version 2 of the License, or
//  (at your option) any later version.
//
//  This program is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.
//
//  You should have received a copy of the GNU General Public License along
//  with this program; if not, write to the Free Software Foundation, Inc.,
//  51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

//! Driver library for controlling the *Vellemann K8055(N)* USB digital and analog IO-cards.
//!
//! See the Vellemann [Homepage](http://www.velleman.eu/products/view/?id=351346) for the
//! hardware specification.

#![crate_type = "lib"]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate error_chain;
extern crate libusb;
extern crate rustc_serialize;

use std::default::Default;
use std::time::Duration;
use libusb::{Context, Device, DeviceHandle};

mod errors {
    use libusb;
    error_chain!{
        foreign_links {
             Usb(libusb::Error);
        }
    }
}

use errors::*;

/// Analog values in the range (0-255).
#[derive(PartialEq, PartialOrd, Debug, Copy, Clone)]
pub enum AnalogChannel {
    A1(u8),
    A2(u8),
}

bitflags!(
#[doc = "
The digital channel values.

Can be combined with bitoperations.
    use k8055::DigitalChannel;
    let dc = D1 & D2 & D3;

See the bitflags documentation for more information.
"]
  pub struct DigitalChannel: u8 {
#[doc = "All flags set to `off`"]
    const DZERO = 0;
    const D1 = 1;
    const D2 = 2;
    const D3 = 4;
    const D4 = 8;
    const D5 = 16;
    const D6 = 32;
    const D7 = 64;
    const D8 = 128;
#[doc = "All flags set to `on`"]
    const DALL = 255;
  }
);

bitflags!(
#[doc = "
Adresses of the different cards that can be controlled.

See the jumper setting on your card for the correct address.
"]
    pub struct CardAddress: u16 {
#[doc = "Use card `0x5500` (see jumper settings)"]
        const CARD_1 = 0x5500;
#[doc = "Use card `0x5501` (see jumper settings)"]
        const CARD_2 = 0x5501;
#[doc = "Use card `0x5502` (see jumper settings)"]
        const CARD_3 = 0x5502;
#[doc = "Use card `0x5503` (see jumper settings)"]
        const CARD_4 = 0x5503;
#[doc = "Automatically selects the first card found on the system"]
        const CARD_ANY = 0x0;
    }
);

const VENDOR_ID: u16 = 0x10cf;

#[derive(Debug)]
enum Packet {
    SetAnalogDigital(u8, u8, u8),
    Status(u8, u8, u8, u8),
}

#[derive(Default)]
struct State {
    dig: u8,
    ana1: u8,
    ana2: u8,
}

/// Object controlling one Vellemann K8055 card.
pub struct K8055<'a> {
    dev: Option<Device<'a>>,
    hd: Option<DeviceHandle<'a>>,
    state: State,
}

impl<'a> K8055<'a> {
    /// Create a new K8055 instance with the first card found on the system.
    ///
    /// May return `None` if no card was found connected to the system.
    pub fn new(ctx: &mut Context) -> Result<K8055> {
        K8055::new_addr(ctx, CardAddress::CARD_ANY)
    }

    /// Create a new K8055 instance with a specific card address.
    ///
    /// See the hardware jumpers on the card for your card's address. May return `None` if no card
    /// with the address `addr` can be found connected to the system.
    pub fn new_addr(ctx: &mut Context, addr: CardAddress) -> Result<K8055> {
        let mut d = None;
        {
            for dev in ctx.devices().unwrap().iter() {
                let desc = try!(
                    dev.device_descriptor()
                        .chain_err(|| "Unable to get device description")
                );
                if addr == CardAddress::CARD_ANY {
                    if desc.vendor_id() == VENDOR_ID
                        && CardAddress::CARD_1.bits == desc.product_id()
                        || CardAddress::CARD_2.bits == desc.product_id()
                        || CardAddress::CARD_3.bits == desc.product_id()
                        || CardAddress::CARD_4.bits == desc.product_id()
                    {
                        d = Some(dev);
                        break;
                    }
                } else if desc.vendor_id() == VENDOR_ID && addr.bits == desc.product_id() {
                    d = Some(dev);
                    break;
                }
            }
        }
        if d.is_some() {
            let k8055 = K8055 {
                dev: d,
                hd: None,
                state: Default::default(),
            };
            Ok(k8055)
        } else {
            Err(libusb::Error::NotFound.into())
        }
    }

    /// Open the device for starting IO operations.
    ///
    /// Returns `true` if the device was successfully opened or is already open. Returns `false`
    /// if the device can't be opened.
    pub fn open(&mut self) -> bool {
        // device already open
        if self.hd.is_some() {
            return true;
        }
        match self.dev {
            Some(ref mut d) => {
                self.hd = d.open().ok();
                true
            }
            None => false,
        }
    }

    /// Set all analog and digital values to zero.
    pub fn reset(&mut self) -> Result<()> {
        self.write(&Packet::SetAnalogDigital(0u8, 0u8, 0u8))
    }

    // digital

    /// Write the digital value `d` to the outports.
    ///
    /// Leaves the analog values untouched. Returns `false` on failure.
    pub fn write_digital_out(&mut self, d: DigitalChannel) -> Result<()> {
        let p = &Packet::SetAnalogDigital(d.bits, self.state.ana1, self.state.ana2);
        self.write(p)
    }

    /// Write the masked digital value `d` to the outports.
    ///
    /// Masks `d` with `mask` to only affect bits which are `on` in the mask.
    /// Leaves the analog values untouched. Returns `false` on failure.
    pub fn write_digital_out_mask(
        &mut self,
        d: DigitalChannel,
        mask: DigitalChannel,
    ) -> Result<()> {
        self.write_digital_out(d & mask)
    }

    /// Return the bits that are currently set on the digital out channel.
    pub fn get_digital_out(&mut self) -> DigitalChannel {
        DigitalChannel::from_bits(self.state.dig).unwrap()
    }

    /// Return the bits that are currently set on the digital out channel, masked with `mask`.
    pub fn get_digital_out_mask(&mut self, d: DigitalChannel) -> DigitalChannel {
        DigitalChannel::from_bits(self.state.dig).unwrap() & d
    }

    /// Read the digital in channel.
    ///
    /// Returns `None` on failure
    pub fn read_digital_in(&mut self) -> Result<DigitalChannel> {
        match self.read() {
            Ok(Packet::Status(dig, _, _, _)) => Ok(DigitalChannel::from_bits(dig).unwrap()),
            Err(e) => Err(e),
            _ => Err(libusb::Error::InvalidParam.into()),
        }
    }

    /// Read the digital in channel masked with `mask`.
    ///
    /// Returns `None` on failure
    pub fn read_digital_in_mask(&mut self, mask: DigitalChannel) -> Result<DigitalChannel> {
        match self.read_digital_in() {
            Ok(c) => Ok(c & mask),
            Err(e) => Err(e),
        }
    }

    // analog
    /// Write the analog value `a` to the given outport.
    ///
    /// Leaves the digital values untouched. Returns `false` on failure.
    pub fn write_analog_out(&mut self, a: AnalogChannel) -> Result<()> {
        let p = match a {
            AnalogChannel::A1(v) => Packet::SetAnalogDigital(self.state.dig, v, self.state.ana2),
            AnalogChannel::A2(v) => Packet::SetAnalogDigital(self.state.dig, self.state.ana1, v),
        };
        self.write(&p)
    }

    /// Return the analog channel 1 out value
    pub fn get_analog_out1(&mut self) -> AnalogChannel {
        AnalogChannel::A1(self.state.ana1)
    }

    /// Return the analog channel 2 out value
    pub fn get_analog_out2(&mut self) -> AnalogChannel {
        AnalogChannel::A2(self.state.ana2)
    }

    /// Read the analog channel 1 input value.
    ///
    /// Returns `None` on failure.
    pub fn read_analog_in1(&mut self) -> Result<AnalogChannel> {
        match self.read() {
            Ok(Packet::Status(_, _, a1, _)) => Ok(AnalogChannel::A1(a1)),
            Err(e) => Err(e),
            _ => Err(libusb::Error::InvalidParam.into()),
        }
    }

    /// Read the analog channel 2 input value.
    ///
    /// Returns `None` on failure.
    pub fn read_analog_in2(&mut self) -> Result<AnalogChannel> {
        match self.read() {
            Ok(Packet::Status(_, _, _, a2)) => Ok(AnalogChannel::A2(a2)),
            Err(e) => Err(e),
            _ => Err(libusb::Error::InvalidParam.into()),
        }
    }

    // private
    fn write(&mut self, p: &Packet) -> Result<()> {
        match self.hd {
            Some(ref mut hd) => {
                let _ = K8055::detach_and_claim(hd);
                let data = try!(K8055::encode(p));

                try!(hd.write_interrupt(0x1, &data, Duration::from_millis(1000)));
                // update the internal state on output changes
                if let Packet::SetAnalogDigital(d, a1, a2) = *p {
                    self.state = State {
                        dig: d,
                        ana1: a1,
                        ana2: a2,
                    };
                    Ok(())
                } else {
                    Err(libusb::Error::InvalidParam.into())
                }
            }
            None => Err(libusb::Error::NoDevice.into()),
        }
    }

    fn read(&mut self) -> Result<Packet> {
        match self.hd {
            Some(ref mut hd) => {
                let _ = K8055::detach_and_claim(hd);
                let mut data = [0u8; 8];
                try!(hd.read_interrupt(0x81, &mut data, Duration::from_millis(1000)));
                K8055::decode(&data)
            }
            None => Err(libusb::Error::NoDevice.into()),
        }
    }

    fn encode(p: &Packet) -> Result<[u8; 8]> {
        match *p {
            Packet::SetAnalogDigital(dig, ana1, ana2) => {
                Ok([5u8, dig, ana1, ana2, 0u8, 0u8, 0u8, 0u8])
            }
            _ => Err(libusb::Error::InvalidParam.into()),
        }
    }

    fn decode(d: &[u8]) -> Result<Packet> {
        Ok(Packet::Status(d[0], d[1], d[2], d[3]))
    }

    fn detach_and_claim(hd: &mut DeviceHandle) -> Result<()> {
        try!(hd.kernel_driver_active(0));
        try!(hd.detach_kernel_driver(0));
        try!(hd.claim_interface(0));
        Ok(())
    }
}

#[test()]
fn find_and_open() {
    let mut ctx = libusb::Context::new().unwrap();
    let mut k = K8055::new(&mut ctx).unwrap();
    assert!(k.open());

    let mut ctx = libusb::Context::new().unwrap();
    assert!(K8055::new_addr(&mut ctx, CardAddress::CARD_2).is_err());
    assert!(K8055::new_addr(&mut ctx, CardAddress::CARD_3).is_err());
    assert!(K8055::new_addr(&mut ctx, CardAddress::CARD_4).is_err());
}

#[test()]
fn write_and_read_digital() {
    use std::thread::sleep;
    use std::time::Duration;

    let mut ctx = libusb::Context::new().unwrap();
    let k = K8055::new(&mut ctx);
    assert!(k.is_ok());
    let mut k = k.unwrap();
    assert!(k.open());
    assert!(k.get_digital_out() == DigitalChannel::DZERO);
    for i in 0..7 {
        //    k.write_digital_out(D1).expect("DO");
        assert!(
            k.write_digital_out(DigitalChannel::from_bits(1u8 << i).unwrap())
                .is_ok()
        );
        assert!(k.get_digital_out() == DigitalChannel::from_bits(1u8 << i).unwrap());
        sleep(Duration::from_millis(100));
    }
    assert!(k.reset().is_ok());
    assert!(k.get_digital_out() == DigitalChannel::DZERO);

    assert!(k.write_digital_out_mask(
        DigitalChannel::D1 | DigitalChannel::D2 | DigitalChannel::D3,
        DigitalChannel::D2
    ).is_ok());
    assert!(k.get_digital_out() == DigitalChannel::D2);
    assert!(k.reset().is_ok());
    sleep(Duration::from_millis(1000));
}

#[test()]
fn write_and_read_analog() {
    use std::thread::sleep;
    use std::time::Duration;

    let mut ctx = libusb::Context::new().unwrap();
    let k = K8055::new(&mut ctx);
    assert!(k.is_ok());
    let mut k = k.unwrap();
    assert!(k.open());
    assert!(k.get_analog_out1() == AnalogChannel::A1(0u8));
    assert!(k.get_analog_out2() == AnalogChannel::A2(0u8));
    for i in 0u8..255 {
        assert!(k.write_analog_out(AnalogChannel::A1(i)).is_ok());
        assert!(k.write_analog_out(AnalogChannel::A2(255 - i)).is_ok());
        sleep(Duration::from_millis(10));
    }
    assert!(k.reset().is_ok());
    sleep(Duration::from_millis(1000));
}
