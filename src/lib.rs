#![crate_type = "lib"]
extern crate libc;
extern crate usb;
extern crate serialize;

use std::io::timer::sleep;
use std::iter::range_inclusive;
use std::fmt::{Show, Formatter, FormatError};
use usb::libusb;


bitflags!(
  flags AnalogChannel: u8 {
      static A1 = 1,
      static A2 = 2
  }
)

bitflags!(
  flags DigitalChannel: u8 { 
    static D1 = 1,
    static D2 = 2,
    static D3 = 4,
    static D4 = 8,
    static D5 = 16,
    static D6 = 32,
    static D7 = 64,
    static D8 = 128
  }
)

#[deriving(Show)]
enum Packet {
    Reset,
    SetAnalogDigital(u8, u8, u8),
    Status(u8)
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

    fn write(&mut self, p: &Packet) -> bool {
        match self.hd {
          Some(ref hd) => {
              unsafe {
                  if libusb::libusb_kernel_driver_active(hd.ptr(), 0) == 1 {
                      if libusb::libusb_detach_kernel_driver(hd.ptr(), 0) != 0 {
                          fail!("Can't detach usb kernel driver");
                      }
                  }
              }
              hd.claim_interface(0u);
              let data = match K8055::encode(p) {
                  Some(d) => d,
                  None => return false
              };
              
              match hd.write(0x1, libusb::LIBUSB_TRANSFER_TYPE_INTERRUPT, data) {
                  Ok(_) => return true,
                  Err(_) => return false
              }
          }
          None => return false
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

    fn decode(d: &[u8, ..8]) -> Option<Packet> {
        //let bytes: Vec<u8> = try!(Decodable::decode(d));
        Some(Status(0u8))
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
  for i in range(0u8, 255) {
    assert!(k.write(&SetAnalogDigital(i, 0u8, 0u8)));
    sleep(100);
  }

  assert!(k.write(&SetAnalogDigital(0u8, 0u8, 0u8)));
}
         

