import bg from '../images/back32.png';

(async () => {
    const wasm = await import("../Cargo.toml")
    const {say_hello, render} = await wasm.default()
    const canvas = document.getElementById('canvas')
    const ctx = canvas.getContext('2d')
    const res = await fetch(bg);
    const img = await loadImage(bg);
    ctx.drawImage(img, 0, 0);
    render(ctx)
    say_hello("dozo")
})()

async function loadImage(url) {
    return new Promise((r) => {
        const i = new Image();
        i.onload = (() => r(i));
        i.src = url;
    });
}