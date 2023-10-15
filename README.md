# Sitix
This is a very simple web templating engine. It is written in Rust for speed and focuses on simplicity and power. It uses the Rasta format for templating.  
TODO: Document Rasta

The recommended method for installing sitix is cargo. `cargo install sitix` should work, assuming you have Rust. Then simply run `sitix <directory>` (directory is optional - `.` will be assumed if it isn't provided). It'll drop the templated files in a new output directory. Files that aren't encoded in valid UTF-8 or don't have a valid Rasta header will be copied raw; any file (regardless of type) encoded in UTF-8 with a valid Rasta header will be templated.
