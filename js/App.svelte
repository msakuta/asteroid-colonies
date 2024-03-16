<script>
    import { onMount } from 'svelte';
    import MessageOverlay from './MessageOverlay.svelte';
    import HeartBeat from './HeartBeat.svelte';
    import SidePanel from './SidePanel.svelte';
    import DebugButton from './DebugButton.svelte';
    import InfoPanel from './InfoPanel.svelte';
    import { websocket, getSessionId, reconnectWebSocket } from './session';
    import BuildMenu from './BuildMenu.svelte';

    export let baseUrl = BASE_URL;
    export let syncPeriod = SYNC_PERIOD;
    export let port = 3883;
    export let serverSync = false;
    export let game = null;

    let infoResult = null;
    let sessionId = null;

    let messageOverlayVisible = false;
    let messageOverlayText = "";

    let heartBroken;
    let heartbeatOpacity = 0;

    let showBuildMenu = false;
    let buildItems = [];
    let buildPos = null;

    let showRecipeMenu = false;

    let mousePos = null;
    let moving = false;
    let movingItem = false;
    let buildingConveyor = null;
    let dragStart = null;
    let dragLast = null;
    let canvas;
    let time = 0;
    let modeName = "";

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
        resizeHandler();
        canvas.addEventListener('pointermove', pointerMove);
        canvas.addEventListener('pointerdown', evt => {
            dragStart = toLogicalCoords(evt.clientX, evt.clientY);
            evt.preventDefault();
            evt.stopPropagation();
        });

        canvas.addEventListener('pointerleave', _ => mousePos = dragStart = null);

        canvas.addEventListener('pointerup', pointerUp);
    });

    function resizeHandler() {
        const bodyRect = document.body.getBoundingClientRect();
        canvas.setAttribute("width", bodyRect.width);
        canvas.setAttribute("height", bodyRect.height);
        game.set_size(bodyRect.width, bodyRect.height);
    }
    window.addEventListener("resize", resizeHandler);

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
            const elem = document.getElementById("conveyor");
            if (!elem?.checked) return;
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
        const buildMenuElem = document.getElementById("buildMenu");
        const recipesElem = document.getElementById("recipes");
        if (name === "move") {
            if (game.start_move_building(x, y)) {
                recipesElem.style.display = "none";
                messageOverlayText = "Choose move building destination";
                messageOverlayVisible = "block";
                moving = true;
            }
        }
        else if (name === "moveItem") {
            if (game.start_move_item(x, y)) {
                recipesElem.style.display = "none";
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
            buildMenuElem.style.display = "none";
            try {
                const recipes = game.get_recipes(x, y);
                while (recipesElem.firstChild) recipesElem.removeChild(recipesElem.firstChild);
                recipesElem.style.display = "block";
                const headerElem = document.createElement("div");
                headerElem.innerHTML = "Select a recipe";
                headerElem.style.fontWeight = "bold";
                recipesElem.appendChild(addCloseButton(() => recipesElem.style.display = "none"));
                recipesElem.appendChild(headerElem);
                const noRecipeElem = document.createElement("div");
                noRecipeElem.innerHTML = `<div class="recipe">No Recipe</div>`;
                noRecipeElem.addEventListener("pointerup", _ => {
                    const [ix, iy] = game.transform_coords(x, y);
                    requestWs("SetRecipe", {pos: [ix, iy]});
                    game.clear_recipe(x, y);
                    recipesElem.style.display = "none";
                });
                recipesElem.appendChild(noRecipeElem);
                for (let recipe of recipes) {
                    const recipeElem = document.createElement("div");
                    const recipeName = recipe.outputs.keys().next().value;
                    recipeElem.innerHTML = formatRecipe(recipe);
                    recipeElem.addEventListener("pointerup", _ => {
                        const [ix, iy] = game.transform_coords(x, y);
                        requestWs("SetRecipe", {pos: [ix, iy], name: recipeName});
                        game.set_recipe(x, y, recipeName);
                        recipesElem.style.display = "none";
                    })
                    recipesElem.appendChild(recipeElem);
                }
            }
            catch (e) {
                console.error(e);
                recipesElem.style.display = "none";
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

    function enterConveyorEdit() {
        const buildMenuElem = document.getElementById("buildMenu");
        const recipesElem = document.getElementById("recipes");
        buildMenuElem.style.display = "none";
        recipesElem.style.display = "none";
        messageOverlayText = "Drag to make build plan and click Ok";
        messageOverlayVisible = true;
        const okButton = document.createElement("button");
        okButton.value = "Ok";
        okButton.innerHTML = "Ok";
        okButton.addEventListener('click', _ => {
            buildingConveyor = null;
            messageOverlayVisible = false;
            const buildPlan = game.commit_build_conveyor(false);
            requestWs("BuildPlan", {build_plan: buildPlan});
        });
        const cancelButton = document.createElement("button");
        cancelButton.value = "Cancel";
        cancelButton.innerHTML = "Cancel";
        cancelButton.addEventListener('click', _ => {
            buildingConveyor = null;
            messageOverlayElem.style.display = "none";
            game.cancel_build_conveyor(false);
        });
        const buttonContainer = document.createElement("div");
        buttonContainer.appendChild(okButton);
        buttonContainer.appendChild(cancelButton);
        messageOverlayElem.appendChild(buttonContainer);
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
        const [ix, iy] = buildPos;
        const type = evt.detail.type;
        requestWs("Build", {pos: buildPos, type});
        game.build(buildPos[0], buildPos[1], type);
        showBuildMenu = false;
    }

    function postChunksDigest() {
        game.uniformify_tiles();
        const chunksDigest = game.serialize_chunks_digest();
        // websocket.send(JSON.stringify({type: "ChunksDigest", payload: {chunks_digest: chunksDigest}}));
        websocket.send(chunksDigest);
    }

    let debugDrawChunks = false;

    function debugClick() {
        debugDrawChunks = !debugDrawChunks;
        game.set_debug_draw_chunks(debugDrawChunks);
    }
</script>

<div class="container" id="container">
    {#if messageOverlayVisible}
        <MessageOverlay text={messageOverlayText} />
    {/if}
    <HeartBeat broken={heartBroken} opacity={heartbeatOpacity}/>
    <canvas bind:this={canvas} id="canvas" width="640" height="480"></canvas>
    <SidePanel bind:radioValue={modeName}/>
    <InfoPanel result={infoResult} />
    {#if showBuildMenu}
        <BuildMenu items={buildItems} on:click={build} on:close={() => showBuildMenu = false}/>
    {/if}
    <DebugButton on:click={debugClick}/>
</div>
