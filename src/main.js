import bg from '../images/back32.png';

(async () => {
    const wasm = await import("../Cargo.toml")
    const {render} = await wasm.default()
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const img = await loadImage(bg);
    render(ctx, img);
})()

async function loadImage(url) {
    return new Promise((r) => {
        const i = new Image();
        i.onload = (() => r(i));
        i.src = url;
    });
}