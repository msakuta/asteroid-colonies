(async () => {
    const wasm = await import("../Cargo.toml")
    const {say_hello} = await wasm.default()
    say_hello("dozo")
  })()