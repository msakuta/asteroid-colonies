import bg from '../images/back32.png';
import power_grid from '../images/power_grid.png';
import conveyor from '../images/conveyor.png';
import power from '../images/power.png';
import excavator from '../images/excavator.png';
import storage from '../images/storage.png';
import crew_cabin from '../images/crew_cabin.png';
import assembler from '../images/assembler.png';
import furnace from '../images/furnace.png';

const canvas = document.getElementById('canvas');

(async () => {
    const wasm = await import("../Cargo.toml")
    const {AsteroidColonies} = await wasm.default();

    const loadImages = [
        ["bg32", bg],
        ["power_grid", power_grid],
        ["conveyor", conveyor],
        ["power", power],
        ["excavator", excavator],
        ["storage", storage],
        ["crew_cabin", crew_cabin],
        ["assembler", assembler],
        ["furnace", furnace],
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
        document.getElementById('info').innerHTML = info;
    });

    canvas.addEventListener('mosueleave', evt => mousePos = null);

    canvas.addEventListener('click', evt => {
        for (let name of ["excavate", "move", "power", "conveyor", "moveItem", "buildPowerPlant", "buildStorage", "recipe"]) {
            const elem = document.getElementById(name);
            if (elem?.checked) {
                const [x, y] = toLogicalCoords(evt.clientX, evt.clientY);
                const recipesElem = document.getElementById("recipes");
                if (name === "recipe") {
                    try {
                        const recipes = game.get_recipes(x, y);
                        while (recipesElem.firstChild) recipesElem.removeChild(recipesElem.firstChild);
                        recipesElem.classList = "recipe";
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
                            let inputs = "";
                            for (let input of recipe.inputs.keys()) {
                                if (inputs) inputs += ", " + input;
                                else inputs += input;
                            }
                            recipeElem.innerHTML = `${recipeName} <= (${inputs})`;
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
            document.getElementById('info').innerHTML = info;
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

function toLogicalCoords(clientX, clientY) {
    const r = canvas.getBoundingClientRect();
    const x = clientX - r.left;
    const y = clientY - r.top;
    return [x, y];
}
