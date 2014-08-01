#![crate_type = "lib"]
extern crate libc;
extern crate usb;
extern crate serialize;

use std::io::timer::sleep;
use std::iter::range_inclusive;
use std::fmt::{Show, Formatter, FormatError};
use std::default::Default;
use usb::libusb;


bitflags!(
  flags AnalogChannel: u8 {
      static A1 = 1,
      static A2 = 2
  }
)

bitflags!(
  flags DigitalChannel: u8 { 
    static Zero = 0,
    static D1 = 1,
    static D2 = 2,
    static D3 = 4,
    static D4 = 8,
    static D5 = 16,
    static D6 = 32,
    static D7 = 64,
    static D8 = 128,
    static All = 255
  }
)

bitflags!(
    flags CardAddress: uint {
        static Card1 = 0x5500,
        static Card2 = 0x5501,
        static Card3 = 0x5502,
        static Card4 = 0x5503,
        static CardAny = 0x0
    }
)

static VendorId: uint = 0x10cfu;

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

pub struct K8055 {
    dev: usb::Device,
    hd: Option<usb::DeviceHandle>,
    state: State    
}

impl K8055 {
    pub fn new() -> Option<K8055> {
        K8055::new_addr(CardAny)
    }

    pub fn new_addr(addr: CardAddress) -> Option<K8055> {
        let c = usb::Context::new();
        let d = if addr == CardAny {
            K8055::find_any_k8055(&c)
        } else {
            c.find_by_vid_pid(VendorId, addr.bits)
        };
        
        if d.is_some() { 
            return Some(K8055{ dev: d.unwrap(), hd: None, state: Default::default() }) 
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
          Err(_) => return false
      }
    }
    
    pub fn reset(&mut self) -> bool {
        self.write(&SetAnalogDigital(0u8, 0u8, 0u8))
    }

    pub fn write_digital_out(&mut self, d: DigitalChannel) -> bool {
        let p = &SetAnalogDigital(d.bits, self.state.ana1, self.state.ana2);
        self.write(p)
    }

    pub fn write_digital_out_mask(&mut self, d: DigitalChannel, mask: DigitalChannel) -> bool {
        self.write_digital_out(d & mask)
    }

    pub fn get_digital_out(&mut self) -> DigitalChannel {
        DigitalChannel::from_bits(self.state.dig).unwrap()
    }
   
    pub fn get_digital_out_mask(&mut self, d: DigitalChannel) -> DigitalChannel {
        DigitalChannel::from_bits(self.state.dig).unwrap() & d
    }

// private 

    fn find_any_k8055(c: &usb::Context) -> Option<usb::Device> {
        for pid in range_inclusive(Card1.bits, Card4.bits) {
          let d = c.find_by_vid_pid(VendorId, pid);
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
            fail!("Can't detach usb kernel driver");
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
  assert!(K8055::new_addr(Card2).is_none());
  assert!(K8055::new_addr(Card3).is_none());
  assert!(K8055::new_addr(Card4).is_none());
}

#[test()]
fn write_and_read_digital() {
  let k = K8055::new();
  assert!(k.is_some());
  let mut k = k.unwrap();
  assert!(k.open());
  assert!(k.get_digital_out() == Zero);
  for i in range(0u, 8) {
    assert!(k.write_digital_out(DigitalChannel::from_bits(1u8<<i).unwrap()));
    assert!(k.get_digital_out() == DigitalChannel::from_bits(1u8<<i).unwrap());
    sleep(100);
  }
  assert!(k.reset());
  assert!(k.get_digital_out() == Zero);

  assert!(k.write_digital_out_mask(D1 | D2 | D3, D2));
  assert!(k.get_digital_out() == D2);
  assert!(k.reset());
}
         

