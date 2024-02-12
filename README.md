# Asteroid Colonies

A HTML5 game that you can build a colony in an asteroid.

In the colony, you can build excavator, smelter, assembler, residential areas and automate processing items.

Try it now on your browser!
https://msakuta.github.io/asteroid-colonies/

## What is this?

Asteroid Colonies is a HTML5 game that is similar to [FactorishWasm](https://github.com/msakuta/FactorishWasm), but targets online multiplayer game in a persistent universe.

**Note that the graphics are temporary!!** They are not finalized yet!

![screenshot](doc/screenshot00.png)

## Technologies

This project uses following technologies:

* Rust 1.74.1 (WebAssembly build)
* node.js 16
* npm 8.3.1
* rollup 4.9.6

## How to build

Install [Rust](https://www.rust-lang.org/tools/install).

Install node.js.

Install npm.

Run

    npm run build

and the game page is deployed in `dist/`.



## How to run development server

Run

    npm run watch

Launch another terminal and run

    cd dist && npx serve

and browse http://localhost:3000/ for development.


## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
