import closeButton from '../images/close.png';
import bg from '../images/back32.png';
import cursor from '../images/cursor.png';
import crew from '../images/crew.png';
import rawOre from '../images/rawOre.png';
import ironIngot from '../images/ironIngot.png';
import copperIngot from '../images/copperIngot.png';
import cilicate from '../images/cilicate.png';
import gear from '../images/gear.png';
import wire from '../images/wire.png';
import circuit from '../images/circuit.png';
import power_grid from '../images/power_grid.png';
import conveyor from '../images/conveyor.png';
import conveyorItem from '../images/conveyor-item.png';
import power from '../images/power.png';
import excavator from '../images/excavator.png';
import storage from '../images/storage.png';
import mediumStorage from '../images/mediumStorage.png';
import crew_cabin from '../images/crew_cabin.png';
import assembler from '../images/assembler.png';
import assemblerComponent from '../images/assemblerComponent.png';
import furnace from '../images/furnace.png';
import construction from '../images/construction.png';
import deconstruction from '../images/deconstruction.png';
import cleanup from '../images/cleanup.png';
import heart from '../images/heart.png';
import brokenHeart from '../images/brokenHeart.png';

const canvas = document.getElementById('canvas');
const serverSync = SERVER_SYNC;
const baseUrl = BASE_URL;
const syncPeriod = SYNC_PERIOD;
const port = 3883;
let websocket = null;
let sessionId = null;
let debugDrawChunks = false;

const heartbeatDiv = document.getElementById("heartbeat");
const heartbeatElem = document.createElement("img");
let heartbeatOpacity = 0;
heartbeatElem.setAttribute("src", heart);
heartbeatDiv.appendChild(heartbeatElem);

(async () => {
    const wasm = await import("../wasm/Cargo.toml");
    const {AsteroidColonies, set_panic_hook} = await wasm.default();

    set_panic_hook();

    const loadImages = [
        ["bg32", bg],
        ["cursor", cursor],
        ["crew", crew],
        ["power_grid", power_grid],
        ["conveyor", conveyor],
        ["power", power],
        ["excavator", excavator],
        ["storage", storage],
        ["medium_storage", mediumStorage],
        ["crew_cabin", crew_cabin],
        ["assembler", assembler],
        ["furnace", furnace],
        ["raw_ore", rawOre],
        ["iron_ingot", ironIngot],
        ["copper_ingot", copperIngot],
        ["cilicate", cilicate],
        ["gear", gear],
        ["wire", wire],
        ["circuit", circuit],
        ["construction", construction],
        ["deconstruction", deconstruction],
        ["cleanup", cleanup],
    ].map(async ([name, src]) => {
        return [name, src, await loadImage(src)];
    });
    const loadedImages = await Promise.all(loadImages);

    const canvasRect = canvas.getBoundingClientRect();
    const game = new AsteroidColonies(loadedImages, canvasRect.width, canvasRect.height);

    if(serverSync && !sessionId){
        const sessionRes = await fetch(`http://${location.hostname}:${port}/api/session`, {
            method: "POST"
        });
        sessionId = await sessionRes.text();
        const dataRes = await fetch(`${baseUrl}/api/load`);
        const dataText = await dataRes.text();
        game.deserialize(dataText);
    }

    if(!websocket){
        reconnectWebSocket();
    }

    function resizeHandler(evt) {
        const bodyRect = document.body.getBoundingClientRect();
        canvas.setAttribute("width", bodyRect.width);
        canvas.setAttribute("height", bodyRect.height);
        game.set_size(bodyRect.width, bodyRect.height);
    }
    window.addEventListener("resize", resizeHandler);
    resizeHandler();
    const ctx = canvas.getContext('2d');
    game.render(ctx);
    let mousePos = null;
    let moving = null;
    let buildingConveyor = null;
    let dragStart = null;
    let dragLast = null;
    const messageOverlayElem = document.getElementById("messageOverlay");

    canvas.addEventListener('pointerdown', evt => {
        dragStart = toLogicalCoords(evt.clientX, evt.clientY);
        evt.preventDefault();
        evt.stopPropagation();
    });

    function pointerMove(evt) {
        const [x, y] = mousePos = toLogicalCoords(evt.clientX, evt.clientY);
        if (!moving) {
            game.set_cursor(x, y);
            const info = game.get_info(x, y);
            document.getElementById('info').innerHTML = formatInfo(info);
        }
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

    canvas.addEventListener('pointermove', pointerMove);

    canvas.addEventListener('pointerleave', _ => mousePos = dragStart = null);

    canvas.addEventListener('pointerup', evt => {
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
                const from = game.transform_coords(moving[0], moving[1]);
                const to = game.transform_coords(x, y);
                requestWs("Move", {from: [from[0], from[1]], to: [to[0], to[1]]});
                game.move_building(moving[0], moving[1], x, y);
            }
            catch (e) {
                console.error(`move_building: ${e}`);
            }
            messageOverlayElem.style.display = "none";
            moving = null;
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

        for (let name of ["excavate", "move", "power", "conveyor", "splitter", "merger", "moveItem", "build", "cancel", "deconstruct", "recipe", "cleanup"]) {
            const elem = document.getElementById(name);
            if (elem?.checked) {
                const buildMenuElem = document.getElementById("buildMenu");
                const recipesElem = document.getElementById("recipes");
                if (name === "move") {
                    recipesElem.style.display = "none";
                    messageOverlayElem.innerHTML = "Choose move destination";
                    messageOverlayElem.style.display = "block";
                    moving = [x, y];
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
                    recipesElem.style.display = "none";
                    try {
                        const buildMenu = game.get_build_menu(x, y);
                        while (buildMenuElem.firstChild) buildMenuElem.removeChild(buildMenuElem.firstChild);
                        buildMenuElem.style.display = "block";
                        const headerElem = document.createElement("div");
                        headerElem.innerHTML = "Select a building";
                        buildMenuElem.appendChild(addCloseButton(() => buildMenuElem.style.display = "none"));
                        headerElem.style.fontWeight = "bold";
                        buildMenuElem.appendChild(headerElem);
                        for (let buildItem of buildMenu) {
                            const buildItemElem = document.createElement("div");
                            const buildingType = buildItem.type_;
                            buildItemElem.innerHTML = formatBuildItem(buildItem);
                            buildItemElem.addEventListener("pointerup", _ => {
                                const [ix, iy] = game.transform_coords(x, y);
                                requestWs("Build", {pos: [ix, iy], type: {Building: buildingType.Building}});
                                game.build(x, y, buildingType.Building);
                                buildMenuElem.style.display = "none";
                            })
                            buildMenuElem.appendChild(buildItemElem);
                        }
                    }
                    catch (e) {
                        console.error(e);
                        buildMenuElem.style.display = "none";
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
                    buildMenuElem.style.display = "none";
                    recipesElem.style.display = "none";
                    if (name === "excavate") {
                        const [ix, iy] = game.transform_coords(x, y);
                        requestWs("Excavate", {x: ix, y: iy});
                    }
                    else if (name === "power") {
                        const [ix, iy] = game.transform_coords(x, y);
                        requestWs("Build", {type: "PowerGrid", pos: [ix, iy]});
                    }
                    if (game.command(name, x, y)) {
                        requestAnimationFrame(() => game.render(ctx));
                    }
                }
                return;
            }
        }
    })

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
        messageOverlayElem.innerHTML = "Drag to make build plan and click Ok";
        messageOverlayElem.style.display = "block";
        const okButton = document.createElement("button");
        okButton.value = "Ok";
        okButton.innerHTML = "Ok";
        okButton.addEventListener('click', _ => {
            buildingConveyor = null;
            messageOverlayElem.style.display = "none";
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

    let time = 0;

    setInterval(async () => {
        // Increment time before any await. Otherwise, this async function runs 2-4 times every tick for some reason.
        time++;
        // if (serverSync && time % syncPeriod === 0) {
        //     console.log(`serverSync period: ${time}`);
        //     const dataRes = await fetch(`${baseUrl}/api/load`);
        //     const dataText = await dataRes.text();
        //     game.deserialize(dataText);
        // }
        game.tick();
        game.render(ctx);
        if (mousePos !== null) {
            const info = game.get_info(mousePos[0], mousePos[1]);
            document.getElementById('info').innerHTML = formatInfo(info);
        }
        if (websocket) {
            heartbeatOpacity = Math.max(0, heartbeatOpacity - 0.2);
            updateHeartbeatOpacity();
        }
    }, 100);

    function reconnectWebSocket(){
        if(sessionId){
            websocket = new WebSocket(`ws://${location.hostname}:${port}/ws/${sessionId}`);
            websocket.binaryType = "arraybuffer";
            websocket.addEventListener("message", (event) => {
                if (event.data instanceof ArrayBuffer) {
                    const byteArray = new Uint8Array(event.data);
                    game.deserialize_bin(byteArray);
                    postChunksDigest();
                    heartbeatOpacity = 1;
                    updateHeartbeatOpacity();
                }
                else {
                // console.log(`Event through WebSocket: ${event.data}`);
                    const data = JSON.parse(event.data);
                    if(data.type === "clientUpdate"){
                        if(game){
                            game.deserialize(data.payload);
                            postChunksDigest();
                            heartbeatOpacity = 1;
                            updateHeartbeatOpacity();
                        }
                        // const payload = data.payload;
                        // const body = CelestialBody.celestialBodies.get(payload.bodyState.name);
                        // if(body){
                        //     body.clientUpdate(payload.bodyState);
                        // }
                    }
                }
            });
            websocket.addEventListener("open", postChunksDigest);
        }
    }

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

    function requestWs(type, payload) {
        if (!websocket) {
            return;
        }
        websocket.send(JSON.stringify({
            type,
            payload
        }));
    }

    function postChunksDigest() {
        game.uniformify_tiles();
        const chunksDigest = game.serialize_chunks_digest();
        // websocket.send(JSON.stringify({type: "ChunksDigest", payload: {chunks_digest: chunksDigest}}));
        websocket.send(chunksDigest);
    }
})()

async function loadImage(url) {
    return new Promise((r) => {
        const i = new Image();
        i.onload = (() => r(i));
        i.src = url;
    });
}

function itemToIcon(item) {
    switch(item){
        case "RawOre": return rawOre;
        case "IronIngot": return ironIngot;
        case "CopperIngot": return copperIngot;
        case "Cilicate": return cilicate;
        case "Gear": return gear;
        case "Wire": return wire;
        case "Circuit": return circuit;
        case "PowerGridComponent": return power_grid;
        case "ConveyorComponent": return conveyorItem;
        case "AssemblerComponent": return assemblerComponent;
    }
}

function buildingToIcon(building) {
    switch(building){
        case "Power": return power;
        case "Excavator": return excavator;
        case "Storage": return storage;
        case "MediumStorage": return mediumStorage;
        case "CrewCabin": return crew_cabin;
        case "Assembler": return assemblerComponent;
        case "Furnace": return furnace;
    }
}

function iconWithCount(itemUrl, count) {
    const widthFactor = 1;
    const heightFactor = 1;
    return `<div class="item" style="
        display: inline-block;
        position: relative;
        background-image: url(${itemUrl});
        background-size: ${32 * widthFactor}px ${32 * heightFactor}px;
        width: 32px;
        height: 32px;
      ">
        <div class="overlay noselect">
        ${count}
        </div>
      </div>`;
}

function iconWithoutCount(itemUrl) {
    const widthFactor = 1;
    const heightFactor = 1;
    return `<div class="item" style="
        display: inline-block;
        position: relative;
        background-image: url(${itemUrl});
        background-size: ${32 * widthFactor}px ${32 * heightFactor}px;
        width: 32px;
        height: 32px;
      "></div>`;
}

function formatBuildItem(buildItem) {
    let inputs = "";
    for (let [input, count] of buildItem.ingredients.entries()) {
        const icon = iconWithCount(itemToIcon(input), count);
        if (inputs) inputs += " " + icon;
        else inputs += icon;
    }
    const {Building: output} = buildItem.type_;
    const icon = iconWithoutCount(buildingToIcon(output));
    return `<div class="recipe">${icon} <= ${inputs}</div>`;
}

function formatRecipe(recipe) {
    let inputs = "";
    for (let [input, count] of recipe.inputs.entries()) {
        const icon = iconWithCount(itemToIcon(input), count);
        if (inputs) inputs += " " + icon;
        else inputs += icon;
    }
    let outputs = "";
    for (let [output, count] of recipe.outputs.entries()) {
        const icon = iconWithCount(itemToIcon(output), count);
        if (outputs) outputs += " " + icon;
        else outputs += icon;
    }
    return `<div class="recipe">${outputs} <= ${inputs}</div>`;
}

function formatInventory(inventory) {
    let result = "";
    for (let [input, count] of inventory.entries()) {
        const icon = iconWithCount(itemToIcon(input), count);
        if (result) result += " " + icon;
        else result += icon;
    }
    return result;
}

function formatCrews(building) {
    return `${building.crews} / ${building.max_crews}`;
}

function formatConstruction(construction) {
    let result = `Type: ${construction.type_}`;
    for (let [input, count] of construction.ingredients.entries()) {
        const icon = iconWithCount(itemToIcon(input), count);
        if (result) result += " " + icon;
        else result += icon;
    }
    return result;
}

function formatInfo(result) {
    return `Building: ${result.building?.type_}
    Task: ${result.building?.task}
    Recipe: ${result.building?.recipe ? formatRecipe(result.building.recipe) : ""}
    Inventory: ${result.building?.inventory ? formatInventory(result.building.inventory) : ""}
    Crews: ${result.building ? formatCrews(result.building) : ""}
    Construction: ${result.construction ? formatConstruction(result.construction) : ""}
    Power capacity: ${result.power_capacity} kW
    Power demand: ${result.power_demand} kW
    Power load: ${(result.power_demand / result.power_capacity * 100).toFixed(1)} %
    Transports: ${result.transports}`;
}

function toLogicalCoords(clientX, clientY) {
    const r = canvas.getBoundingClientRect();
    const x = clientX - r.left;
    const y = clientY - r.top;
    return [x, y];
}

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
