# k8055.rs

Rust support for the Vellemann K8055 IO Card

[![Build Status](https://travis-ci.org/hirschenberger/k8055.rs.svg?branch=master)](https://travis-ci.org/hirschenberger/k8055.rs)
[![](http://meritbadge.herokuapp.com/k8055)](https://crates.io/crates/k8055)
[![License](http://img.shields.io/:license-MIT-blue.svg)](http://doge.mit-license.org)

## Testing

To run the tests, attach your Vellemann k8055 card, jumpered as `CARD1`. You must run the tests sequentially to avoid
interrupting each other and return unexpected values:

```
RUST_TEST_THREADS=1 cargo test
```

## License
Copyright © 2015-2019 Falco Hirschenberger

Distributed under the [MIT License](LICENSE).

