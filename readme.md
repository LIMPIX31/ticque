# ticque

A pattern that can help request data from a data stream running in a loop

## Example
```rust
use ticque::{Vendor, Customer};

let vendor = Vendor::new();
let consumer = vendor.consumer();

tokio::spawn(async move {
  let camera_stream = ...;

  loop {
    let frame = camera_stream.next().await?;

    if vendor.has_waiters() {
      let rgb_image = frame_to_rgb(frame)?;
      vendor.send(rgb_image);
    }
  }
});

let current_rgb_image = consumer.request().await?;
```
