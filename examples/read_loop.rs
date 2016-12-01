extern crate libusb;
extern crate k8055;
use k8055::K8055;


fn main() {
 if let Ok(mut ctx) = libusb::Context::new(){
 {
     match K8055::new(&mut ctx) {
       Ok(ref mut k) => {
         k.open();
         let mut n = 10;
         let mut old = k8055::DZERO;
         loop {
             if n == 0 { break; }
             let new = k.read_digital_in().unwrap();
             if new != old {
                 old = new;
                 k.write_digital_out(new).expect("Error writing DO");
                 n -=1;
             }
         }
     },
     Err(e) => panic!(e)
   }
}
 }
}
