<script>
    import { onMount } from 'svelte';
    import MessageOverlay from './MessageOverlay.svelte';
    import HeartBeat from './HeartBeat.svelte';
    import SidePanel from './SidePanel.svelte';
    import DebugButton from './DebugButton.svelte';
    import InfoPanel from './InfoPanel.svelte';
    import { websocket, fetchSessionId, reconnectWebSocket } from './session';
    import BuildMenu from './BuildMenu.svelte';
    import RecipeMenu from './RecipeMenu.svelte';

    export let baseUrl = BASE_URL;
    export let port = 3883;
    export let serverSync = false;
    export let game = null;

    let infoResult = null;

    let messageOverlayVisible = false;
    let messageOverlayText = "";
    let messageShowOk = false;
    let messageShowCancel = false;

    let heartBroken;
    let heartbeatOpacity = 0;

    let showBuildMenu = false;
    let buildItems = [];
    let buildPos = null;

    let showRecipeMenu = false;
    let recipeItems = [];
    let recipePos = null;

    let mousePos = null;
    let moving = false;
    let movingItem = false;
    let buildingConveyor = null;
    let dragStart = null;
    let dragLast = null;
    let canvas;
    let time = 0;
    let modeName = "";

    let reconnectTime = 0;
    let websocketOptions = {
        port,
        game,
        onupdate: () => {
            heartbeatOpacity = 1;
            updateHeartbeatOpacity();
        },
    };

    if(serverSync){
        heartBroken = true;
        heartbeatOpacity = 1;
        fetchSessionId({
            port,
            baseUrl,
            game,
        })
        .then(() => reconnectWebSocket(websocketOptions));
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
            try {
                game.preview_build_conveyor(buildingConveyor[0], buildingConveyor[1], x, y, true);
            }
            catch (e) {
                console.error(`build_conveyor: ${e}`);
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
        resizeHandler();
        canvas.addEventListener('pointermove', pointerMove);
        canvas.addEventListener('pointerdown', evt => {
            dragStart = toLogicalCoords(evt.clientX, evt.clientY);
            evt.preventDefault();
            evt.stopPropagation();
        });

        canvas.addEventListener('pointerleave', _ => mousePos = dragStart = null);

        canvas.addEventListener('pointerup', pointerUp);
        window.addEventListener("resize", resizeHandler);
    });

    function resizeHandler() {
        const bodyRect = document.body.getBoundingClientRect();
        canvas.setAttribute("width", bodyRect.width);
        canvas.setAttribute("height", bodyRect.height);
        game.set_size(bodyRect.width, bodyRect.height);
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
            if (websocket.readyState === 1) {
                heartbeatOpacity = Math.max(0, heartbeatOpacity - 0.2);
            }
            updateHeartbeatOpacity();
            if (websocket.readyState === 3 && reconnectTime-- <= 0) {
                reconnectWebSocket(websocketOptions);
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
                heartbeatOpacity = 1;
                break;
        }
    }

    function setRecipe(evt) {
        const recipeName = evt.detail.type;
        const [x, y] = recipePos;
        requestWs("SetRecipe", {pos: [x, y], name: recipeName});
        game.set_recipe(x, y, recipeName);
        showRecipeMenu = false;
    }

    function clearRecipe() {
        const [x, y] = recipePos;
        requestWs("SetRecipe", {pos: [x, y]});
        game.clear_recipe(x, y);
        showRecipeMenu = false;
    }

    function pointerUp(evt) {
        const [x, y] = toLogicalCoords(evt.clientX, evt.clientY);
        if (dragStart) {
            dragStart = null;
            if (dragLast) {
                dragLast = null;
                evt.preventDefault();
                return;
            }
        }
        if (moving) {
            try {
                const to = game.transform_coords(x, y);
                const from = game.move_building(x, y);
                requestWs("Move", {from: [from[0], from[1]], to: [to[0], to[1]]});
            }
            catch (e) {
                console.error(`move_building: ${e}`);
            }
            messageOverlayVisible = false;
            moving = false;
            return;
        }
        if (movingItem) {
            try {
                const to = game.transform_coords(x, y);
                const from = game.move_item(x, y);
                requestWs("MoveItem", {from: [from[0], from[1]], to: [to[0], to[1]]});
            }
            catch (e) {
                console.error(`move_item: ${e}`);
            }
            messageOverlayVisible = false;
            movingItem = false;
            return;
        }

        if (buildingConveyor) {
            try {
                game.preview_build_conveyor(buildingConveyor[0], buildingConveyor[1], x, y, false);
                buildingConveyor = [x, y];
            }
            catch (e) {
                console.error(`build_conveyor: ${e}`);
            }
            return;
        }

        const name = modeName;
        if (name === "move") {
            if (game.start_move_building(x, y)) {
                showBuildMenu = false;
                showRecipeMenu = false;
                messageOverlayText = "Choose move building destination";
                messageOverlayVisible = "block";
                moving = true;
            }
        }
        else if (name === "moveItem") {
            if (game.start_move_item(x, y)) {
                showBuildMenu = false;
                showRecipeMenu = false;
                messageOverlayText = "Choose move item destination";
                messageOverlayVisible = "block";
                movingItem = true;
            }
        }
        else if (name === "conveyor") {
            enterConveyorEdit();
            buildingConveyor = [x, y];
        }
        else if (name === "splitter") {
            enterConveyorEdit();
            game.build_splitter(x, y);
        }
        else if (name === "merger") {
            enterConveyorEdit();
            game.build_merger(x, y);
        }
        else if (name === "build") {
            showRecipeMenu = false;
            try {
                const buildMenu = game.get_build_menu(x, y);
                buildItems = buildMenu;
                showBuildMenu = true;
                buildPos = game.transform_coords(x, y);
            }
            catch (e) {
                console.error(e);
                showBuildMenu = false;
            }
        }
        else if (name === "recipe") {
            showBuildMenu = false;
            try {
                recipeItems = game.get_recipes(x, y);
                showRecipeMenu = true;
                recipePos = game.transform_coords(x, y);
            }
            catch (e) {
                console.error(e);
                showRecipeMenu = false;
            }
        }
        else if (name === "cancel") {
            const pos = game.transform_coords(x, y);
            requestWs("CancelBuild", {pos: [pos[0], pos[1]]});
            game.cancel_build(x, y);
        }
        else if (name === "deconstruct") {
            const pos = game.transform_coords(x, y);
            requestWs("Deconstruct", {pos: [pos[0], pos[1]]});
            game.deconstruct(x, y);
        }
        else if (name === "cleanup") {
            const pos = game.transform_coords(x, y);
            requestWs("Cleanup", {pos: [pos[0], pos[1]]});
            game.cleanup_item(x, y);
        }
        else {
            showBuildMenu = false;
            showRecipeMenu = false;
            if (name === "excavate") {
                const [ix, iy] = game.transform_coords(x, y);
                requestWs("Excavate", {x: ix, y: iy});
            }
            else if (name === "power") {
                const [ix, iy] = game.transform_coords(x, y);
                requestWs("Build", {type: "PowerGrid", pos: [ix, iy]});
            }
            if (game.command(name, x, y)) {
                const ctx = canvas.getContext('2d');
                requestAnimationFrame(() => game.render(ctx));
            }
        }
    }

    document.body.addEventListener("keydown", evt => {
        switch (evt.code) {
            case "KeyD":
                debugDrawChunks = !debugDrawChunks;
                game.set_debug_draw_chunks(debugDrawChunks);
                break;
        }
    });

    function conveyorOk() {
        buildingConveyor = null;
        messageOverlayVisible = false;
        const buildPlan = game.commit_build_conveyor(false);
        requestWs("BuildPlan", {build_plan: buildPlan});
    }

    function conveyorCancel() {
        buildingConveyor = null;
        messageOverlayVisible = false;
        game.cancel_build_conveyor(false);
    }

    function enterConveyorEdit() {
        showBuildMenu = false;
        showRecipeMenu = false;
        messageOverlayText = "Drag to make build plan and click Ok";
        messageOverlayVisible = true;
        messageShowOk = true;
        messageShowCancel = true;
    }

    function requestWs(type, payload) {
        if (!websocket) {
            return;
        }
        websocket.send(JSON.stringify({
            type,
            payload
        }));
    }

    function build(evt) {
        const [x, y] = buildPos;
        const type = evt.detail.type;
        requestWs("Build", {pos: [x, y], type: {Building: type}});
        game.build(x, y, type);
        showBuildMenu = false;
    }

    let debugDrawChunks = false;

    function debugClick() {
        debugDrawChunks = !debugDrawChunks;
        game.set_debug_draw_chunks(debugDrawChunks);
    }
</script>

<div class="container">
    {#if messageOverlayVisible}
        <MessageOverlay
            text={messageOverlayText}
            showOkButton={messageShowOk}
            showCancelButton={messageShowCancel}
            on:ok={conveyorOk}
            on:cancel={conveyorCancel}/>
    {/if}
    <HeartBeat broken={heartBroken} opacity={heartbeatOpacity}/>
    <canvas bind:this={canvas} id="canvas" width="640" height="480"></canvas>
    <SidePanel bind:radioValue={modeName}/>
    <InfoPanel result={infoResult} />
    {#if showBuildMenu}
        <BuildMenu items={buildItems} on:click={build} on:close={() => showBuildMenu = false}/>
    {/if}
    {#if showRecipeMenu}
        <RecipeMenu items={recipeItems}
            on:click={setRecipe}
            on:clear={clearRecipe}
            on:close={() => showRecipeMenu = false}/>
    {/if}
    <DebugButton on:click={debugClick}/>
</div>

<style>
.container {
    position: relative;
    margin: 0;
    padding: 0;
}
</style>
