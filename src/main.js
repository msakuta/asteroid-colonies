import bg from '../images/back32.png';
import power from '../images/power.png';

const canvas = document.getElementById('canvas');

(async () => {
    const wasm = await import("../Cargo.toml")
    const {AsteroidColonies} = await wasm.default();

    const game = new AsteroidColonies();
    const ctx = canvas.getContext('2d');
    const img = await loadImage(bg);
    const img2 = await loadImage(power);
    game.render(ctx, img, img2);

    canvas.addEventListener('mousemove', evt => {
        const [x, y] = toLogicalCoords(evt.clientX, evt.clientY);
        const info = game.get_info(x, y);
        document.getElementById('info').innerHTML = info;
    });

    canvas.addEventListener('click', evt => {
        const [x, y] = toLogicalCoords(evt.clientX, evt.clientY);
        if (game.excavate(x, y)) {
            requestAnimationFrame(() => game.render(ctx, img, img2));
        }
    })
})()

async function loadImage(url) {
    return new Promise((r) => {
        const i = new Image();
        i.onload = (() => r(i));
        i.src = url;
    });
}

function toLogicalCoords(clientX, clientY) {
    const r = canvas.getBoundingClientRect();
    const x = clientX - r.left;
    const y = clientY - r.top;
    return [x, y];
}
