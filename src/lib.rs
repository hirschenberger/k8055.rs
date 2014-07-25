#![crate_type = "lib"]
extern crate usb;
extern crate serialize;

use serialize::{Encoder, Encodable, Decoder, Decodable};
use std::iter::range_inclusive;
use std::fmt::{Show, Formatter, FormatError};
use usb::libusb;


#[deriving(Show)]
pub enum AnalogChannel {
    A1,
    A2
}

#[deriving(Show)]
pub enum DigitalChannel {
    D1 = 1,
    D2 = 2,
    D3 = 4,
    D4 = 8,
    D5 = 16,
    D6 = 32,
    D7 = 64,
    D8 = 128
}

#[deriving(Show)]
enum Packet {
    Reset,
    SetAnalogDigital(u8, u8, u8),
    Status(u8)
}

impl<E: Encoder<S>, S> Encodable<E, S> for Packet {
  fn encode(&self, e: &mut E) -> Result<(), S> {
      match *self {
          Reset => [0u8, ..8].encode(e),
          SetAnalogDigital(dig, ana1, ana2) => [5u8, dig, ana1, ana2, 
                                                0u8, 0u8, 0u8, 0u8].encode(e),
          _ => fail!("Unknown cmd")
      }
  }
}

impl<E, D: Decoder<E>> Decodable<D, E> for Packet {
    fn decode(d: &mut D) -> Result<Packet, E> {
        let bytes: Vec<u8> = try!(Decodable::decode(d));
        Ok(Status(0u8))
    }
}

pub struct K8055 {
    dev: usb::Device,
    hd: Option<usb::DeviceHandle>

}

impl K8055 {
    pub fn new() -> Option<K8055> {
        let c = usb::Context::new();
        let d = K8055::find_k8055(&c);
        if d.is_some() { 
            return Some(K8055{ dev: d.unwrap(), hd: None }) 
        } else {
            return None
        }
    }

    pub fn open(&mut self) -> bool {
      // device already open
      if self.hd.is_some() { return true }
      match self.dev.open() {
          Ok(h) => {              
              self.hd = Some(h);
              return true
          }
          Err(e) => return false
      }
    }
    
    fn find_k8055(c: &usb::Context) -> Option<usb::Device> {
        let vid = 0x10cfu;
        for pid in range_inclusive(0x5500, 0x5503) {
          let d = c.find_by_vid_pid(vid, pid);
          if d.is_some() { return d }
        }
        None
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
}
         

