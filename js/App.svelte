<script>
    import { onMount } from 'svelte';
    import MessageOverlay from './MessageOverlay.svelte';
    import HeartBeat from './HeartBeat.svelte';
    // import SidePanel from './SidePanel.svelte';
    import ButtonFrames from './ButtonFrames.svelte';
    import DebugButton from './DebugButton.svelte';
    import InfoPanel from './InfoPanel.svelte';
    import { websocket, fetchSessionId, reconnectWebSocket, tickTime } from './session';
    import BuildMenu from './BuildMenu.svelte';
    import RecipeMenu from './RecipeMenu.svelte';
    import ErrorMessage from './ErrorMessage.svelte';
    import RadialMenu from './RadialMenu.svelte';
    import excavateIcon from '../images/excavate.png';
    import moveBuildingIcon from '../images/moveBuilding.png';
    import recipeIcon from '../images/recipe.png';
    import buildIcon from '../images/build.png';
    import buildPowerGridIcon from '../images/buildPowerGrid.png';
    import buildConveyorIcon from '../images/buildConveyor.png';
    import buildSplitterIcon from '../images/buildSplitter.png';
    import buildMergerIcon from '../images/buildMerger.png';
    import moveItemIcon from '../images/moveItem.png';
    import buildBuildingIcon from '../images/buildBuilding.png';
    import cancelBuildIcon from '../images/cancelBuild.png';
    import deconstructIcon from '../images/deconstruct.png';
    import cleanup from '../images/cleanup.png';
    import { loadAllIcons } from './graphics';

    export let baseUrl = BASE_URL;
    export let port = 3883;
    export let serverSync = false;
    export let game = null;

    let infoResult = null;

    let messageOverlayVisible = false;
    let messageOverlayText = "";
    let messageShowOk = false;
    let messageShowCancel = false;

    let showErrorMessage = false;
    let errorMessage = "";
    let errorMessageTimeout = 0;

    let heartBroken;
    let heartbeatOpacity = 0;

    let showBuildMenu = false;
    let buildItems = [];
    let buildPos = null;

    let showRecipeMenu = false;
    let recipeItems = [];
    let recipePos = null;

    const useWebGL = true;

    let buttons = [
        {mode: 'excavate', icon: excavateIcon},
        {mode: 'move', icon: moveBuildingIcon},
        {mode: 'power', icon: buildPowerGridIcon},
        {mode: 'conveyor', icon: buildConveyorIcon},
        {mode: 'splitter', icon: buildSplitterIcon},
        {mode: 'merger', icon: buildMergerIcon},
        {mode: 'moveItem', icon: moveItemIcon},
        {mode: 'build', icon: buildBuildingIcon},
        {mode: 'recipe', icon: recipeIcon},
        {mode: 'cancel', icon: cancelBuildIcon},
        {mode: 'deconstruct', icon: deconstructIcon},
        {mode: 'cleanup', icon: cleanup},
    ];

    const RADIAL_MENU_MAIN = [
        {caption: "Excavate", event: 'excavate', icon: excavateIcon},
        {caption: "Move Bldg.", event: 'moveBuilding', icon: moveBuildingIcon},
        {caption: "Build", event: 'buildMenu', icon: buildIcon},
        {caption: "Set Recipe", event: 'setRecipe', icon: recipeIcon},
        {caption: "Move Item", event: 'moveItem', icon: moveItemIcon},
        {caption: "Cleanup", event: 'cleanup', icon: cleanup},
    ];
    const RADIAL_MENU_BUILD = [
        {caption: "Power Grid", event: 'buildPowerGrid', icon: buildPowerGridIcon},
        {caption: "Conveyor", event: 'buildConveyor', icon: buildConveyorIcon},
        {caption: "Splitter", event: 'buildSplitter', icon: buildSplitterIcon},
        {caption: "Merger", event: 'buildMerger', icon: buildMergerIcon},
        {caption: "Building", event: 'buildBuilding', icon: buildBuildingIcon},
        {caption: "Deconstruct", event: 'deconstruct', icon: deconstructIcon},
    ];
    let showRadialMenu = false;
    let radialScreenPos = null;
    let radialPos = null;

    let mousePos = null;
    let moving = false;
    let movingItem = false;
    let buildingConveyor = null;
    let dragStart = null;
    let dragLast = null;
    let fingerDist = null;
    let activePointers = [];
    let zoomChanging = false;
    let canvas;
    let time = 0;
    let modeName = "";


    let reconnectTime = 0;
    let websocketOptions = {
        baseUrl,
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
                const [ix, iy] = game.transform_coords(x, y);
                game.preview_build_conveyor(buildingConveyor[0], buildingConveyor[1], ix, iy, true);
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

    // Multi-touch event tracking.
    // see https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events/Multi-touch_interaction
    function updatePointerEvent(evt) {
        // Remove this event from the target's cache
        const index = activePointers.findIndex(
            (cachedEv) => cachedEv.pointerId === evt.pointerId,
        );
        if (0 <= index) {
            activePointers[index] = evt;
        }
    }

    function removeEvent(evt) {
        // Remove this event from the target's cache
        const index = activePointers.findIndex(
            (cachedEv) => cachedEv.pointerId === evt.pointerId,
        );
        if (0 <= index) {
            activePointers.splice(index, 1);
        }
        if (activePointers.length <= 1) {
            fingerDist = null;
        }
        if (activePointers.length === 0) {
            zoomChanging = false;
        }
    }

    onMount(async () => {
        if (useWebGL) {
            const images = await loadAllIcons();
            const gl = canvas.getContext('webgl', { alpha: false });
            game.load_gl_assets(gl, images);
        }
        resizeHandler();

        canvas.addEventListener('pointermove', evt => {
            updatePointerEvent(evt);
            if (1 < activePointers.length) {
                zoomChanging = true;
                const newFingerDist = getMultitouchDistance();
                if (fingerDist !== null) {
                    const [x, y] = toLogicalCoords(evt.clientX, evt.clientY);
                    const scale = 1 / Math.abs(fingerDist / newFingerDist);
                    game.set_zoom(x, y, scale);
                }
                fingerDist = newFingerDist;
            }
            if (!zoomChanging) {
                pointerMove(evt);
            }
        });
        canvas.addEventListener('pointerdown', evt => {
            dragStart = toLogicalCoords(evt.clientX, evt.clientY);
            activePointers.push(evt);
            evt.preventDefault();
            evt.stopPropagation();
        });

        canvas.addEventListener('pointerleave', evt => {
            mousePos = dragStart = null;
            removeEvent(evt);
        });

        canvas.addEventListener('pointerup', evt => {
            if (!zoomChanging) {
                wrapErrorMessage(evt => pointerUpInt(evt))(evt);
            }
            removeEvent(evt);
        });

        function getMultitouchDistance() {
            const diffX = activePointers[0].clientX - activePointers[1].clientX;
            const diffY = activePointers[0].clientY - activePointers[1].clientY;
            return Math.sqrt(diffX * diffX + diffY * diffY);
        }

        window.addEventListener("resize", resizeHandler);
        window.addEventListener("wheel", evt => {
            const [x, y] = toLogicalCoords(evt.clientX, evt.clientY);
            game.change_zoom(x, y, evt.deltaY);
        });

        // Don't start timer until the assets are loaded, otherwise an error will be thrown
        requestAnimationFrame(frameProc);
    });

    function resizeHandler() {
        const bodyRect = document.body.getBoundingClientRect();
        canvas.setAttribute("width", bodyRect.width);
        canvas.setAttribute("height", bodyRect.height);
        const gl = canvas.getContext("webgl");
        gl.viewport(0, 0, canvas.width, canvas.height);
        game.set_size(bodyRect.width, bodyRect.height);
    }

    let lastUpdated = null;
    let lastShowed = null;

    // Usually, a tick is much shorter than a frame.
    // If the user puts the browser tab into background, it may become dormant and
    // takes longer. We limit the number of frames to catch up in that case.
    const MAX_TICKS_PER_FRAME = 10;

    function frameProc() {
        // Increment time before any await. Otherwise, this async function runs 2-4 times every tick for some reason.
        time++;
        // if (serverSync && time % syncPeriod === 0) {
        //     console.log(`serverSync period: ${time}`);
        //     const dataRes = await fetch(`${baseUrl}/api/load`);
        //     const dataText = await dataRes.text();
        //     game.deserialize(dataText);
        // }
        const now = performance.now();
        const deltaTime = (now - lastShowed) / 1000;
        if (lastUpdated === null) {
            lastUpdated = now;
        }
        if (MAX_TICKS_PER_FRAME * tickTime < deltaTime) {
            console.log(`Skipping ${((now - lastUpdated) / 1000 / tickTime).toFixed(0)} frames`);
            lastUpdated = now - tickTime * MAX_TICKS_PER_FRAME * 1000;
        }
        while (tickTime < (now - lastUpdated) / 1000) {
            lastUpdated += tickTime * 1000;
            game.tick();
        }
        if (useWebGL) {
            const gl = canvas.getContext('webgl', { alpha: false });
            // gl.clearColor(0., 0.5, 0., 1.);
            // gl.clear(gl.COLOR_BUFFER_BIT);
            game.render_gl(gl, (now - lastUpdated) / tickTime / 1000, performance.now() / 1000);
        }
        else {
            const ctx = canvas.getContext('2d');
            game.render(ctx);
        }
        if (mousePos !== null) {
            const info = game.get_info(mousePos[0], mousePos[1]);
            infoResult = info;
        }
        if (websocket) {
            if (websocket.readyState === 1) {
                heartbeatOpacity = Math.max(0, heartbeatOpacity - deltaTime);
            }
            updateHeartbeatOpacity();
            if (websocket.readyState === 3 && reconnectTime-- <= 0) {
                reconnectWebSocket(websocketOptions);
                // Randomize retry time in attempt to avoid contention
                reconnectTime = Math.floor(Math.random() * 50) + 10;
            }
        }
        if (showErrorMessage) {
            errorMessageTimeout = errorMessageTimeout - deltaTime;
            if(errorMessageTimeout < 0) {
                showErrorMessage = false;
            }
        }
        lastShowed = now;
        requestAnimationFrame(frameProc);
    }

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

    function pointerUpInt(evt) {
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
            const to = game.transform_coords(x, y);
            try {
                const from = game.move_building(x, y);
                requestWs("Move", {from: [from[0], from[1]], to: [to[0], to[1]]});
            }
            finally {
                messageOverlayVisible = false;
                moving = false;
            }
            return;
        }
        if (movingItem) {
            try {
                const to = game.transform_coords(x, y);
                const from = game.move_item(x, y);
                requestWs("MoveItem", {from: [from[0], from[1]], to: [to[0], to[1]]});
            }
            finally {
                messageOverlayVisible = false;
                movingItem = false;
            }
            return;
        }

        if (buildingConveyor) {
            const [ix, iy] = game.transform_coords(x, y);
            game.preview_build_conveyor(buildingConveyor[0], buildingConveyor[1], ix, iy, false);
            buildingConveyor = [ix, iy];
            return;
        }

        // Make sure to set cursor for touch panels.
        // Mouse doesn't need to set cursor here, because it always has
        // the current position updated by pointermove event, but
        // touch devices do not.
        game.set_cursor(x, y);

        const name = modeName;
        if (name === "move") {
            const [ix, iy] = game.transform_coords(x, y);
            game.start_move_building(ix, iy);
            showBuildMenu = false;
            showRecipeMenu = false;
            messageOverlayText = "Choose move building destination";
            messageOverlayVisible = "block";
            moving = true;
        }
        else if (name === "moveItem") {
            const [ix, iy] = game.transform_coords(x, y);
            startMoveItem(ix, iy);
        }
        else if (name === "conveyor") {
            enterConveyorEdit();
            buildingConveyor = [x, y];
        }
        else if (name === "splitter") {
            enterConveyorEdit();
            const [ix, iy] = game.transform_coords(x, y);
            game.build_splitter(ix, iy);
        }
        else if (name === "merger") {
            enterConveyorEdit();
            const [ix, iy] = game.transform_coords(x, y);
            game.build_merger(ix, iy);
        }
        else if (name === "build") {
            showBuildBuildingMenu(game.transform_coords(x, y));
        }
        else if (name === "recipe") {
            setShowRecipeMenu(x, y);
        }
        else if (name === "cancel") {
            const pos = game.transform_coords(x, y);
            requestWs("CancelBuild", {pos: [pos[0], pos[1]]});
            game.cancel_build(x, y);
        }
        else if (name === "deconstruct") {
            const pos = game.transform_coords(x, y);
            requestWs("Deconstruct", {pos: [pos[0], pos[1]]});
            game.deconstruct(pos[0], pos[1]);
        }
        else if (name === "cleanup") {
            const pos = game.transform_coords(x, y);
            requestWs("Cleanup", {pos: [pos[0], pos[1]]});
            game.cleanup_item(x, y);
        }
        else {
            showBuildMenu = false;
            showRecipeMenu = false;
            const [ix, iy] = game.transform_coords(x, y);
            if (name === "excavate") {
                requestWs("Excavate", {x: ix, y: iy});
            }
            else if (name === "power") {
                requestWs("Build", {type: "PowerGrid", pos: [ix, iy]});
            }
            else {
                positionRadialMenu(x, y);
                RADIAL_MENU_MAIN[0].grayed = !game.is_excavatable_at(ix, iy);
                RADIAL_MENU_MAIN[1].grayed =
                RADIAL_MENU_MAIN[3].grayed = !game.find_building(ix, iy);
                showRadialMenu = RADIAL_MENU_MAIN;
                radialPos = game.transform_coords(x, y);
                return;
            }
            if (game.command(name, x, y)) {
                const ctx = canvas.getContext('2d');
                requestAnimationFrame(() => game.render(ctx));
            }
        }
    }

    function wrapErrorMessage(f) {
        return evt => {
            try {
                f(evt);
            }
            catch (e) {
                errorMessage = e;
                showErrorMessage = true;
                errorMessageTimeout = 3;
            }
        };
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

    function positionRadialMenu(x, y) {
        const bodyRect = document.body.getBoundingClientRect();
        const [max, min] = [Math.max, Math.min];
        const margin = 128;
        radialScreenPos = [
            max(margin, min(bodyRect.width - margin, x)),
            max(margin, min(bodyRect.height - margin, y))
        ];
    }

    let commandExcavate = wrapErrorMessage(() => {
        showRadialMenu = false;
        const [x, y] = radialPos;
        game.excavate(x, y);
        requestWs("Excavate", {x: x, y: y});
    });

    let commandMoveBuilding = wrapErrorMessage(() => {
        const [x, y] = radialPos;
        showRadialMenu = false;
        showBuildMenu = false;
        showRecipeMenu = false;
        game.start_move_building(x, y);
        messageOverlayText = "Choose move building destination";
        messageOverlayVisible = "block";
        moving = true;
    });

    function buildMenu(evt) {
        let [x, y] = radialScreenPos;
        if (game.find_construction(radialPos[0], radialPos[1])) {
            RADIAL_MENU_BUILD[5].grayed = false;
            RADIAL_MENU_BUILD[5].caption = "Cancel Build";
            RADIAL_MENU_BUILD[5].event = "cancelBuild";
            RADIAL_MENU_BUILD[5].icon = cancelBuildIcon;
        }
        else {
            RADIAL_MENU_BUILD[5].grayed = !game.find_building(radialPos[0], radialPos[1]);
            RADIAL_MENU_BUILD[5].caption = "Deconstruct";
            RADIAL_MENU_BUILD[5].event = "deconstruct";
            RADIAL_MENU_BUILD[5].icon = deconstructIcon;
        }
        showRadialMenu = RADIAL_MENU_BUILD;
        positionRadialMenu(x + 64, y);
    }

    let buildPowerGrid = wrapErrorMessage(() => {
        showRadialMenu = false;
        let [x, y] = radialPos;
        game.build_power_grid(x, y);
        requestWs("Build", {type: "PowerGrid", pos: [x, y]});
    });

    function buildConveyor() {
        showRadialMenu = false;
        let [x, y] = radialPos;
        enterConveyorEdit();
        buildingConveyor = [x, y];
    }

    function buildSplitter() {
        showRadialMenu = false;
        let [x, y] = radialPos;
        enterConveyorEdit();
        game.build_splitter(x, y);
    }

    function buildMerger() {
        showRadialMenu = false;
        let [x, y] = radialPos;
        enterConveyorEdit();
        game.build_merger(x, y);
    }

    function showBuildBuildingMenu(pos) {
        showRadialMenu = false;
        showRecipeMenu = false;
        try {
            const buildMenu = game.get_build_menu();
            buildItems = buildMenu;
            showBuildMenu = true;
            buildPos = pos;
        }
        catch (e) {
            console.error(e);
            showBuildMenu = false;
        }
    }

    function commandBuildBuildingMenu(evt) {
        showBuildBuildingMenu(radialPos);
    }

    let commandDeconstruct = wrapErrorMessage(() => {
        showRadialMenu = false;
        const [x, y] = radialPos;
        requestWs("Deconstruct", {pos: [x, y]});
        game.deconstruct(x, y);
    });

    let commandCancelBuild = wrapErrorMessage(() => {
        showRadialMenu = false;
        const [x, y] = radialPos;
        requestWs("CancelBuild", {pos: [x, y]});
        game.cancel_build(x, y);
    });

    let commandBuild = wrapErrorMessage((evt) => {
        const [x, y] = buildPos;
        const type = evt.detail.type;
        requestWs("Build", {pos: [x, y], type: {Building: type}});
        game.build(x, y, type);
        showBuildMenu = false;
    });

    function setShowRecipeMenu(x, y) {
        showRadialMenu = false;
        showBuildMenu = false;
        recipePos = [x, y];
        recipeItems = game.get_recipes(x, y);
        showRecipeMenu = true;
    }

    let commandRecipeShow = wrapErrorMessage(() => {
        showRadialMenu = false;
        const [x, y] = radialPos;
        setShowRecipeMenu(x, y);
    });

    function startMoveItem(x, y) {
        if (game.start_move_item(x, y)) {
            showBuildMenu = false;
            showRecipeMenu = false;
            messageOverlayText = "Choose move item destination";
            messageOverlayVisible = "block";
            movingItem = true;
        }
    }

    let commandMoveItem = wrapErrorMessage(() => {
        showRadialMenu = false;
        const [x, y] = radialPos;
        startMoveItem(x, y);
    });

    let commandCleanup = wrapErrorMessage(() => {
        showRadialMenu = false;
        const [x, y] = radialPos;
        requestWs("Cleanup", {pos: [x, y]});
        game.cleanup_item(x, y);
    });

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
    <!-- <SidePanel bind:radioValue={modeName}/> -->
    <ButtonFrames bind:modeName={modeName} buttons={buttons}/>
    <InfoPanel result={infoResult} />
    {#if showBuildMenu}
        <BuildMenu items={buildItems} on:click={commandBuild} on:close={() => showBuildMenu = false}/>
    {/if}
    {#if showRecipeMenu}
        <RecipeMenu items={recipeItems}
            on:click={setRecipe}
            on:clear={clearRecipe}
            on:close={() => showRecipeMenu = false}/>
    {/if}
    {#if showRadialMenu === RADIAL_MENU_MAIN}
        <RadialMenu
            pos={radialScreenPos}
            items={showRadialMenu}
            on:close={() => showRadialMenu = false}
            on:excavate={commandExcavate}
            on:moveBuilding={commandMoveBuilding}
            on:buildMenu={buildMenu}
            on:setRecipe={commandRecipeShow}
            on:moveItem={commandMoveItem}
            on:cleanup={commandCleanup}/>
    {:else if showRadialMenu === RADIAL_MENU_BUILD}
        <RadialMenu
            centerIcon={buildIcon}
            pos={radialScreenPos}
            items={showRadialMenu}
            on:close={() => showRadialMenu = false}
            on:buildPowerGrid={buildPowerGrid}
            on:buildConveyor={buildConveyor}
            on:buildBuilding={commandBuildBuildingMenu}
            on:buildSplitter={buildSplitter}
            on:buildMerger={buildMerger}
            on:deconstruct={commandDeconstruct}
            on:cancelBuild={commandCancelBuild}/>
    {/if}
    {#if showErrorMessage}
        <ErrorMessage text={errorMessage} timeout={errorMessageTimeout} on:click={() => showErrorMessage = false}/>
    {/if}
    <DebugButton on:click={debugClick}/>
</div>

<style>
.container {
    position: relative;
    margin: 0;
    padding: 0;
}

#canvas {
    display: block;
    touch-action: none;
    width: 100%;
    height: 100%;
}
</style>
