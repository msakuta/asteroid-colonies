import bg from '../images/back32.png';
import power from '../images/power.png';

(async () => {
    const wasm = await import("../Cargo.toml")
    const {AsteroidColonies} = await wasm.default();

    const game = new AsteroidColonies();
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');
    const img = await loadImage(bg);
    const img2 = await loadImage(power);
    game.render(ctx, img, img2);

    canvas.addEventListener('mousemove', evt => {
        const r = canvas.getBoundingClientRect();
        const x = evt.clientX - r.left;
        const y = evt.clientY - r.top;
        const info = game.get_info(x, y);
        document.getElementById('info').innerHTML = info;
    });
})()

async function loadImage(url) {
    return new Promise((r) => {
        const i = new Image();
        i.onload = (() => r(i));
        i.src = url;
    });
}