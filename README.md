# p2p-chat

A toy p2p chat protocol implementation inspired partly by [Hypercore](https://github.com/datproject/docs/blob/master/papers/dat-paper.pdf) with local [mDNS discovery](https://en.wikipedia.org/wiki/Multicast_DNS), append-only log data structure and replication protocol for learning purposes.

Please note: *This is work in progress and will be published together with a tutorial when finished.*

## Usage

Start chat channel:

  ```
  cargo run
  > chat://20d7eb0934d482fca4f975270b8ad6e28ecbdeebad5bed8c1acd5006eec771ea
  ```

Join existing chat channel:

  ```
  cargo run -- --channel chat://20d7eb0934d482fca4f975270b8ad6e28ecbdeebad5bed8c1acd5006eec771ea
  ```
