
import App from './App.svelte';
import { loadAllIcons } from './graphics.js';

const serverSync = SERVER_SYNC;
const baseUrl = BASE_URL;
const syncPeriod = SYNC_PERIOD;
const port = 3883;

(async () => {
    const wasm = await import("../wasm/Cargo.toml");
    const {AsteroidColonies, set_panic_hook} = await wasm.default();

    set_panic_hook();

    const loadedImages = await loadAllIcons();

    const game = new AsteroidColonies(loadedImages, 640, 480);
    let app = new App({
        target: document.body,
        props: {
            baseUrl,
            syncPeriod,
            port,
            serverSync,
            game,
        }
    });
})()

function addCloseButton(onclose) {
    const closeButtonElem = document.createElement("span");
    closeButtonElem.style.position = "absolute";
    closeButtonElem.style.right = '5px';
    closeButtonElem.style.top = '5px';
    closeButtonElem.style.width = '16px';
    closeButtonElem.style.height = '16px';
    closeButtonElem.style.backgroundImage = `url(${closeButton})`;
    closeButtonElem.addEventListener('click', onclose);
    return closeButtonElem;
}
