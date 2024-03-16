<script>
    import HeartBeat from './HeartBeat.svelte';
    import SidePanel from './SidePanel.svelte';
    import DebugButton from './DebugButton.svelte';

    export let game = null;

    let heartbeatOpacity = 0;

    function updateHeartbeatOpacity() {
        if(!websocket) return;
        switch (websocket.readyState) {
            case 1:
                heartbeatElem.setAttribute("src", heart);
                heartbeatDiv.style.opacity = heartbeatOpacity;
                heartbeatDiv.style.display = 0 < heartbeatOpacity ? "block" : "none";
                break;
            case 3:
                heartbeatElem.setAttribute("src", brokenHeart);
                heartbeatDiv.style.opacity = 1;
                heartbeatDiv.style.display = "block";
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
    <HeartBeat/>
    <canvas id="canvas" width="640" height="480"></canvas>
    <SidePanel/>
    <DebugButton on:click={debugClick}/>
</div>
