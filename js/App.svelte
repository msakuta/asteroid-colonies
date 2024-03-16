<script>
    import HeartBeat from './HeartBeat.svelte';
    import SidePanel from './SidePanel.svelte';
    import DebugButton from './DebugButton.svelte';
    import InfoPanel from './InfoPanel.svelte';
    import { websocket, getSessionId, reconnectWebSocket } from './session';

    export let baseUrl = BASE_URL;
    export let syncPeriod = SYNC_PERIOD;
    export let port = 3883;
    export let serverSync = false;
    export let game = null;

    let infoResult = null;
    let sessionId = null;

    let heartBroken;
    let heartbeatOpacity = 0;

    let mousePos = null;
    let canvas;
    let time = 0;

    if(serverSync && !sessionId){
        getSessionId(id => sessionId = id);
    }

    setInterval(() => {
        // Increment time before any await. Otherwise, this async function runs 2-4 times every tick for some reason.
        time++;
        // if (serverSync && time % syncPeriod === 0) {
        //     console.log(`serverSync period: ${time}`);
        //     const dataRes = await fetch(`${baseUrl}/api/load`);
        //     const dataText = await dataRes.text();
        //     game.deserialize(dataText);
        // }
        const ctx = canvas.getContext('2d');
        game.tick();
        game.render(ctx);
        if (mousePos !== null) {
            const info = game.get_info(mousePos[0], mousePos[1]);
            infoResult = info;
        }
        if (websocket) {
            heartbeatOpacity = Math.max(0, heartbeatOpacity - 0.2);
            updateHeartbeatOpacity();
            if (websocket.readyState === 3 && reconnectTime-- <= 0) {
                reconnectWebSocket();
                // Randomize retry time in attempt to avoid contention
                reconnectTime = Math.floor(Math.random() * 50) + 10;
            }
        }
    }, 100);

    function updateHeartbeatOpacity() {
        if(!websocket) return;
        switch (websocket.readyState) {
            case 1:
                heartBroken = false;
                break;
            case 3:
                heartBroken = true;
                heartbeatDiv.style.opacity = 1;
                break;
        }
    }

    let debugDrawChunks = false;

    function debugClick() {
        debugDrawChunks = !debugDrawChunks;
        game.set_debug_draw_chunks(debugDrawChunks);
    }
</script>

<div class="container" id="container">
    <HeartBeat broken={heartBroken} opacity={heartbeatOpacity}/>
    <canvas bind:this={canvas} id="canvas" width="640" height="480"></canvas>
    <SidePanel/>
    <InfoPanel result={infoResult}/>
    <DebugButton on:click={debugClick}/>
</div>
