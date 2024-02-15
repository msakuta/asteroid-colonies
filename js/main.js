import bg from '../images/back32.png';
import cursor from '../images/cursor.png';
import rawOre from '../images/rawOre.png';
import ironIngot from '../images/ironIngot.png';
import copperIngot from '../images/copperIngot.png';
import cilicate from '../images/cilicate.png';
import gear from '../images/gear.png';
import wire from '../images/wire.png';
import circuit from '../images/circuit.png';
import power_grid from '../images/power_grid.png';
import conveyor from '../images/conveyor.png';
import power from '../images/power.png';
import excavator from '../images/excavator.png';
import storage from '../images/storage.png';
import mediumStorage from '../images/mediumStorage.png';
import crew_cabin from '../images/crew_cabin.png';
import assembler from '../images/assembler.png';
import assemblerComponent from '../images/assemblerComponent.png';
import furnace from '../images/furnace.png';
import construction from '../images/construction.png';

const canvas = document.getElementById('canvas');

(async () => {
    const wasm = await import("../Cargo.toml");
    const {AsteroidColonies, set_panic_hook} = await wasm.default();

    set_panic_hook();

    const loadImages = [
        ["bg32", bg],
        ["cursor", cursor],
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
    ].map(async ([name, src]) => {
        return [name, src, await loadImage(src)];
    });
    const loadedImages = await Promise.all(loadImages);

    const canvasRect = canvas.getBoundingClientRect();
    const game = new AsteroidColonies(loadedImages, canvasRect.width, canvasRect.height);
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
    let dragStart = null;
    let dragLast = null;

    canvas.addEventListener('pointerdown', evt => {
        dragStart = toLogicalCoords(evt.clientX, evt.clientY);
        evt.preventDefault();
    });

    function pointerMove(evt) {
        const [x, y] = mousePos = toLogicalCoords(evt.clientX, evt.clientY);
        if (!moving) {
            game.set_cursor(x, y);
            const info = game.get_info(x, y);
            document.getElementById('info').innerHTML = formatInfo(info);
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
        const messageOverlayElem = document.getElementById("messageOverlay");
        if (moving) {
            try {
                game.move_building(moving[0], moving[1], x, y);
            }
            catch (e) {
                console.error(`move_building: ${e}`);
            }
            messageOverlayElem.style.display = "none";
            moving = null;
            return;
        }

        for (let name of ["excavate", "move", "power", "conveyor", "moveItem", "build", "recipe"]) {
            const elem = document.getElementById(name);
            if (elem?.checked) {
                const buildMenuElem = document.getElementById("buildMenu");
                const recipesElem = document.getElementById("recipes");
                if (name === "move") {
                    recipesElem.style.display = "none";
                    try {
                        messageOverlayElem.innerHTML = "Choose move destination";
                        messageOverlayElem.style.display = "block";
                        moving = [x, y];
                    }
                    catch (e) {
                        console.error(e);
                        messageOverlayElem.style.display = "none";
                    }
                }
                else if (name === "build") {
                    recipesElem.style.display = "none";
                    try {
                        const buildMenu = game.get_build_menu(x, y);
                        while (buildMenuElem.firstChild) buildMenuElem.removeChild(buildMenuElem.firstChild);
                        buildMenuElem.style.position = "absolute";
                        buildMenuElem.style.display = "block";
                        buildMenuElem.style.left = `${x}px`;
                        buildMenuElem.style.top = `${y}px`;
                        const headerElem = document.createElement("div");
                        headerElem.innerHTML = "Select a building";
                        headerElem.style.fontWeight = "bold";
                        buildMenuElem.appendChild(headerElem);
                        for (let buildItem of buildMenu) {
                            const buildItemElem = document.createElement("div");
                            const buildingType = buildItem.type_;
                            buildItemElem.innerHTML = formatBuildItem(buildItem);
                            buildItemElem.addEventListener("click", evt => {
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
                        recipesElem.appendChild(headerElem);
                        for (let recipe of recipes) {
                            const recipeElem = document.createElement("div");
                            const recipeName = recipe.outputs.keys().next().value;
                            recipeElem.innerHTML = formatRecipe(recipe);
                            recipeElem.addEventListener("click", evt => {
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
                else {
                    buildMenuElem.style.display = "none";
                    recipesElem.style.display = "none";
                    if (game.command(name, x, y)) {
                        requestAnimationFrame(() => game.render(ctx));
                    }
                }
                return;
            }
        }
    })

    let time = 0;

    setInterval(() => {
        game.tick();
        game.render(ctx);
        if (mousePos !== null) {
            const info = game.get_info(mousePos[0], mousePos[1]);
            document.getElementById('info').innerHTML = formatInfo(info);
        }
        time++;
    }, 100);
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
        case "ConveyorComponent": return conveyor;
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
