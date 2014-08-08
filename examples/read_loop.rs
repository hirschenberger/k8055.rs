extern crate k8055;
use k8055::{K8055, DigitalChannel};

fn main() {
  let mut k = K8055::new().unwrap();
  k.open();

  let mut n = 10u;
  let mut old = k8055::DZero;
  loop {
      if n == 0 { break; }
      let new = k.read_digital_in().unwrap();
      if new != old {
          old = new;
          k.write_digital_out(new);
          n -=1;
      }
  }
}
