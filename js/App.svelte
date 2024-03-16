<script>
    import { onMount } from 'svelte';
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
    let moving = false;
    let movingItem = false;
    let buildingConveyor = null;
    let dragStart = null;
    let dragLast = null;
    let canvas;
    let time = 0;

    if(serverSync && !sessionId){
        getSessionId({
            port,
            baseUrl,
            game,
            setSessionId: id => sessionId = id,
        });
    }

    function toLogicalCoords(clientX, clientY) {
        const r = canvas.getBoundingClientRect();
        const x = clientX - r.left;
        const y = clientY - r.top;
        return [x, y];
    }

    function pointerMove(evt) {
        const [x, y] = mousePos = toLogicalCoords(evt.clientX, evt.clientY);
        game.set_cursor(x, y);
        infoResult = game.get_info(x, y);
        if (buildingConveyor) {
            const elem = document.getElementById("conveyor");
            if (elem?.checked) {
                try {
                    game.preview_build_conveyor(buildingConveyor[0], buildingConveyor[1], x, y, true);
                }
                catch (e) {
                    console.error(`build_conveyor: ${e}`);
                }
            }
        }
        if (dragStart) {
            if (dragLast) {
                game.pan(x - dragLast[0], y - dragLast[1]);
                dragLast = [x, y];
            }
            else {
                const dragDX = Math.abs(x - dragStart[0]);
                const dragDY = Math.abs(y - dragStart[1]);
                // Determine mouse drag (or panning with a touch panel) or a click (or a tap) by checking moved distance
                if (10 < Math.max(dragDX, dragDY)) {
                    dragLast = dragStart;
                }
            }
        }
    }

    onMount(() => {
        canvas.addEventListener('pointermove', pointerMove);
    });

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
            console.log(`${info}`);
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
