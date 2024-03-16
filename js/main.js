
import App from './App.svelte';
import { loadAllIcons } from './graphics.js';

const serverSync = SERVER_SYNC;
const baseUrl = BASE_URL;
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
            port,
            serverSync,
            game,
        }
    });
})()
