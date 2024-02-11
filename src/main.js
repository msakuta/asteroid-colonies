import bg from '../images/back32.png';
import rawOre from '../images/rawOre.png';
import ironIngot from '../images/ironIngot.png';
import copperIngot from '../images/copperIngot.png';
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

const canvas = document.getElementById('canvas');

(async () => {
    const wasm = await import("../Cargo.toml");
    const {AsteroidColonies, set_panic_hook} = await wasm.default();

    set_panic_hook();

    const loadImages = [
        ["bg32", bg],
        ["power_grid", power_grid],
        ["conveyor", conveyor],
        ["power", power],
        ["excavator", excavator],
        ["storage", storage],
        ["medium_storage", mediumStorage],
        ["crew_cabin", crew_cabin],
        ["assembler", assembler],
        ["furnace", furnace],
        ["iron_ingot", ironIngot],
        ["copper_ingot", copperIngot]
    ].map(async ([name, src]) => {
        return [name, src, await loadImage(src)];
    });
    const loadedImages = await Promise.all(loadImages);

    const game = new AsteroidColonies(loadedImages);
    const ctx = canvas.getContext('2d');
    game.render(ctx);
    let mousePos = null;

    canvas.addEventListener('mousemove', evt => {
        const [x, y] = mousePos = toLogicalCoords(evt.clientX, evt.clientY);
        const info = game.get_info(x, y);
        document.getElementById('info').innerHTML = formatInfo(info);
    });

    canvas.addEventListener('mouseleave', evt => mousePos = null);

    canvas.addEventListener('click', evt => {
        for (let name of ["excavate", "move", "power", "conveyor", "moveItem", "buildPowerPlant", "buildStorage", "buildAssembler", "recipe"]) {
            const elem = document.getElementById(name);
            if (elem?.checked) {
                const [x, y] = toLogicalCoords(evt.clientX, evt.clientY);
                const recipesElem = document.getElementById("recipes");
                if (name === "recipe") {
                    try {
                        const recipes = game.get_recipes(x, y);
                        while (recipesElem.firstChild) recipesElem.removeChild(recipesElem.firstChild);
                        recipesElem.style.position = "absolute";
                        recipesElem.style.display = "block";
                        recipesElem.style.left = `${x}px`;
                        recipesElem.style.top = `${y}px`;
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
                        container.appendChild(recipesElem);
                    }
                    catch (e) {
                        console.error(e);
                        recipesElem.style.display = "none";
                    }
                }
                else {
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
        case "Gear": return gear;
        case "Wire": return wire;
        case "Circuit": return circuit;
        case "PowerGridComponent": return power_grid;
        case "ConveyorComponent": return conveyor;
        case "AssemblerComponent": return assemblerComponent;
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

function formatInfo(result) {
    return `Building: ${result.building?.type_}
    Recipe: ${result.building?.recipe ? formatRecipe(result.building.recipe) : ""}
    Inventory: ${result.building?.inventory ? formatInventory(result.building.inventory) : ""}
    Power capacity: ${result.power_capacity} kW
    Used power: ${result.power_consumed} kW
    Transports: ${result.transports}`;
}

function toLogicalCoords(clientX, clientY) {
    const r = canvas.getBoundingClientRect();
    const x = clientX - r.left;
    const y = clientY - r.top;
    return [x, y];
}
