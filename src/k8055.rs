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
extern crate libc;
extern crate usb;
extern crate serialize;

use std::iter::range_inclusive;
use std::fmt::{Show, Formatter, FormatError};
use std::default::Default;
use usb::libusb;

/// Analog values in the range (0-255).
#[deriving(PartialEq, PartialOrd, Show)]
pub enum AnalogChannel {
      A1(u8),
      A2(u8)
}

bitflags!(
#[doc = "
The digital channel values.

Can be combined with bitoperations.

    let dc = k8055::D1 & k8055::D2 & k8055::D3;

See the bitflags documentation for more information.
"]
  flags DigitalChannel: u8 {
#[doc = "All flags set to `off`"]
    const DZERO = 0,
    const D1 = 1,
    const D2 = 2,
    const D3 = 4,
    const D4 = 8,
    const D5 = 16,
    const D6 = 32,
    const D7 = 64,
    const D8 = 128,
#[doc = "All flags set to `on`"]
    const DALL = 255
  }
)

bitflags!(
#[doc = "
Adresses of the different cards that can be controlled.

See the jumper setting on your card for the correct address.
"]
    flags CardAddress: uint {
#[doc = "Use card `0x5500` (see jumper settings)"]
        const CARD_1 = 0x5500,
#[doc = "Use card `0x5501` (see jumper settings)"]
        const CARD_2 = 0x5501,
#[doc = "Use card `0x5502` (see jumper settings)"]
        const CARD_3 = 0x5502,
#[doc = "Use card `0x5503` (see jumper settings)"]
        const CARD_4 = 0x5503,
#[doc = "Automatically selects the first card found on the system"]
        const CARD_ANY = 0x0
    }
)

static VENDOR_ID: uint = 0x10cfu;

#[deriving(Show)]
enum Packet {
    Reset,
    SetAnalogDigital(u8, u8, u8),
    Status(u8, u8, u8, u8)
}

#[deriving(Default)]
struct State {
    dig: u8,
    ana1: u8,
    ana2: u8
}

/// Object controlling one Vellemann K8055 card.
pub struct K8055 {
    dev: usb::Device,
    hd: Option<usb::DeviceHandle>,
    state: State
}

impl K8055 {
    /// Create a new K8055 instance with the first card found on the system.
    ///
    /// May return `None` if no card was found connected to the system.
    pub fn new() -> Option<K8055> {
        K8055::new_addr(CARD_ANY)
    }

    /// Create a new K8055 instance with a specific card address.
    ///
    /// See the hardware jumpers on the card for your card's address. May return `None` if no card
    /// with the address `addr` can be found connected to the system.
    pub fn new_addr(addr: CardAddress) -> Option<K8055> {
        let c = usb::Context::new();
        let d = if addr == CARD_ANY {
            K8055::find_any_k8055(&c)
        } else {
            c.find_by_vid_pid(VENDOR_ID, addr.bits)
        };

        if d.is_some() {
            return Some(K8055{ dev: d.unwrap(), hd: None, state: Default::default() })
        } else {
            return None
        }
    }

    /// Open the device for starting IO operations.
    ///
    /// Returns `true` if the device was successfully opened or is already open. Returns `false`
    /// if the device can't be opened.
    pub fn open(&mut self) -> bool {
      // device already open
      if self.hd.is_some() { return true }
      match self.dev.open() {
          Ok(h) => {
              self.hd = Some(h);
              return true
          }
          Err(_) => return false
      }
    }

    /// Set all analog and digital values to zero.
    pub fn reset(&mut self) -> bool {
        self.write(&SetAnalogDigital(0u8, 0u8, 0u8))
    }

// digital

    /// Write the digital value `d` to the outports.
    ///
    /// Leaves the analog values untouched. Returns `false` on failure.
    pub fn write_digital_out(&mut self, d: DigitalChannel) -> bool {
        let p = &SetAnalogDigital(d.bits, self.state.ana1, self.state.ana2);
        self.write(p)
    }

    /// Write the masked digital value `d` to the outports.
    ///
    /// Masks `d` with `mask` to only affect bits which are `on` in the mask.
    /// Leaves the analog values untouched. Returns `false` on failure.
    pub fn write_digital_out_mask(&mut self, d: DigitalChannel, mask: DigitalChannel) -> bool {
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
    pub fn read_digital_in(&mut self) -> Option<DigitalChannel> {
        match self.read() {
            Some(Status(dig, _, _, _)) => Some(DigitalChannel::from_bits(dig).unwrap()),
            _ => None
        }
    }

    /// Read the digital in channel masked with `mask`.
    ///
    /// Returns `None` on failure
    pub fn read_digital_in_mask(&mut self, mask: DigitalChannel) -> Option<DigitalChannel> {
        match self.read_digital_in() {
            Some(c) => Some(c & mask),
            _ => None
        }
    }

// analog
    /// Write the analog value `a` to the given outport.
    ///
    /// Leaves the digital values untouched. Returns `false` on failure.
    pub fn write_analog_out(&mut self, a: AnalogChannel) -> bool {
      let p = match a {
          A1(v) => SetAnalogDigital(self.state.dig, v, self.state.ana2),
          A2(v) => SetAnalogDigital(self.state.dig, self.state.ana1, v)
      };
      self.write(&p)
    }

    /// Return the analog channel 1 out value
    pub fn get_analog_out1(&mut self) -> AnalogChannel {
        A1(self.state.ana1)
    }

    /// Return the analog channel 2 out value
    pub fn get_analog_out2(&mut self) -> AnalogChannel {
        A2(self.state.ana2)
    }

    /// Read the analog channel 1 input value.
    ///
    /// Returns `None` on failure.
    pub fn read_analog_in1(&mut self) -> Option<AnalogChannel> {
        match self.read() {
            Some(Status(_, _, a1, _)) => Some(A1(a1)),
            _ => None
        }
    }

    /// Read the analog channel 2 input value.
    ///
    /// Returns `None` on failure.
    pub fn read_analog_in2(&mut self) -> Option<AnalogChannel> {
        match self.read() {
            Some(Status(_, _, _, a2)) => Some(A2(a2)),
            _ => None
        }
    }

// private
    fn find_any_k8055(c: &usb::Context) -> Option<usb::Device> {
        for pid in range_inclusive(CARD_1.bits, CARD_4.bits) {
          let d = c.find_by_vid_pid(VENDOR_ID, pid);
          if d.is_some() { return d }
        }
        None
    }

    fn write(&mut self, p: &Packet) -> bool {
        match self.hd {
          Some(ref hd) => {
              K8055::detach_and_claim(hd);
              let data = match K8055::encode(p) {
                  Some(d) => d,
                  None => return false
              };

              match hd.write(0x1, libusb::LIBUSB_TRANSFER_TYPE_INTERRUPT, data) {
                  Ok(_) => {
                      // update the internal state on output changes
                      match *p {
                          SetAnalogDigital(d, a1, a2) => {
                              self.state = State{dig: d, ana1: a1, ana2: a2};
                          }
                          _ => ()
                      }
                      return true
                  }
                  Err(_) => return false
              }
          }
          None => return false
        }
    }

    fn read(&mut self) -> Option<Packet> {
        match self.hd {
          Some(ref hd) => {
            K8055::detach_and_claim(hd);
            match hd.read(0x81, libusb::LIBUSB_TRANSFER_TYPE_INTERRUPT, 8) {
              Ok(data) => K8055::decode(data.as_slice()),
              Err(_) => None
            }
          }
          None => None
        }
    }

    fn encode(p: &Packet) -> Option<[u8, ..8]> {
      match *p {
          Reset => Some([0u8, ..8]),
          SetAnalogDigital(dig, ana1, ana2) => Some([5u8, dig, ana1, ana2,
                                                     0u8, 0u8, 0u8, 0u8]),
          _ => None
      }
    }

    fn decode(d: &[u8]) -> Option<Packet> {
       match d {
          [dig, st, ana1, ana2, _, _, _, _] => Some(Status(dig, st, ana1, ana2)),
          _ => None
       }
    }

    fn detach_and_claim(hd: &usb::DeviceHandle ) {
      unsafe {
        if libusb::libusb_kernel_driver_active(hd.ptr(), 0) == 1 {
          if libusb::libusb_detach_kernel_driver(hd.ptr(), 0) != 0 {
            panic!("Can't detach usb kernel driver");
          }
        }
      }
      hd.claim_interface(0u);
    }

}

impl Show for K8055 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FormatError> {
      write!(f, "K8055( bus: {}, address: {} )", self.dev.bus(), self.dev.address())
    }
}


#[test()]
fn find_and_open() {
  let k = K8055::new();
  assert!(k.is_some());
  let mut k = k.unwrap();
  assert!(k.open());
  assert!(K8055::new_addr(CARD_2).is_none());
  assert!(K8055::new_addr(CARD_3).is_none());
  assert!(K8055::new_addr(CARD_4).is_none());
}

#[test()]
fn write_and_read_digital() {
  use std::io::timer::sleep;

  let k = K8055::new();
  assert!(k.is_some());
  let mut k = k.unwrap();
  assert!(k.open());
  assert!(k.get_digital_out() == DZERO);
  for i in range(0u, 8) {
    assert!(k.write_digital_out(DigitalChannel::from_bits(1u8<<i).unwrap()));
    assert!(k.get_digital_out() == DigitalChannel::from_bits(1u8<<i).unwrap());
    sleep(100);
  }
  assert!(k.reset());
  assert!(k.get_digital_out() == DZERO);

  assert!(k.write_digital_out_mask(D1 | D2 | D3, D2));
  assert!(k.get_digital_out() == D2);
  assert!(k.reset());
  sleep(1000);
}

#[test()]
fn write_and_read_analog() {
  use std::io::timer::sleep;

  let k = K8055::new();
  assert!(k.is_some());
  let mut k = k.unwrap();
  assert!(k.open());
  assert!(k.get_analog_out1() == A1(0u8));
  assert!(k.get_analog_out2() == A2(0u8));
  for i in range(0u8, 255) {
    assert!(k.write_analog_out(A1(i)));
    assert!(k.write_analog_out(A2(255-i)));
    sleep(10);
  }
  assert!(k.reset());
  sleep(1000);
}
