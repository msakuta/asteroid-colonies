import bg from '../images/back32.png';
import power from '../images/power.png';

(async () => {
    const wasm = await import("../Cargo.toml")
    const {render, AsteroidColonies} = await wasm.default();

    const game = new AsteroidColonies();
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const img = await loadImage(bg);
    const img2 = await loadImage(power);
    game.render(ctx, img, img2);
})()

async function loadImage(url) {
    return new Promise((r) => {
        const i = new Image();
        i.onload = (() => r(i));
        i.src = url;
    });
}