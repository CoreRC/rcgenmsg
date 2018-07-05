# rcgenmsg

[![Build Status](https://travis-ci.org/CoreRC/rcgenmsg.svg?branch=master)](https://travis-ci.org/CoreRC/rcgenmsg)

The message definition transpiler for ROS.

# Caveats

- The generator will generate a message ID deterministically.
- Inline comments are preserved, however, due to the inherent arbitrary nature
  of ROS message definitions, it is not possible to reliably determine the
  association of out-of-line comments to values.

# LICENSE

MIT/Apache 2.0