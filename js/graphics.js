import closeButton from '../images/close.png';
import bg from '../images/back32.png';
import cursor from '../images/cursor.png';
import moveCursor from '../images/moveCursor.png';
import crew from '../images/crew.png';
import rawOre from '../images/rawOre.png';
import ironIngot from '../images/ironIngot.png';
import copperIngot from '../images/copperIngot.png';
import lithiumIngot from '../images/lithiumIngot.png';
import cilicate from '../images/cilicate.png';
import gear from '../images/gear.png';
import wire from '../images/wire.png';
import circuit from '../images/circuit.png';
import batteryItem from '../images/batteryItem.png';
import power_grid from '../images/power_grid.png';
import conveyor from '../images/conveyor.png';
import conveyorItem from '../images/conveyor-item.png';
import atomicBattery from '../images/atomicBattery.png';
import battery from '../images/battery.png';
import batteryBuilding from '../images/batteryBuilding.png';
import excavator from '../images/excavator.png';
import excavatorItem from '../images/excavatorItem.png';
import storage from '../images/storage.png';
import mediumStorage from '../images/mediumStorage.png';
import crewCabin from '../images/crewCabin.png';
import assembler from '../images/assembler.png';
import assemblerComponent from '../images/assemblerComponent.png';
import furnace from '../images/furnace.png';
import furnaceItem from '../images/furnaceItem.png';
import construction from '../images/construction.png';
import deconstruction from '../images/deconstruction.png';
import cleanup from '../images/cleanup.png';
import heart from '../images/heart.png';
import brokenHeart from '../images/brokenHeart.png';
import debug from '../images/debug.png';


export function itemToIcon(item) {
    switch(item){
        case "RawOre": return rawOre;
        case "IronIngot": return ironIngot;
        case "CopperIngot": return copperIngot;
        case "LithiumIngot": return lithiumIngot;
        case "Cilicate": return cilicate;
        case "Gear": return gear;
        case "Wire": return wire;
        case "Circuit": return circuit;
        case "Battery": return batteryItem;
        case "PowerGridComponent": return power_grid;
        case "ConveyorComponent": return conveyorItem;
        case "AssemblerComponent": return assemblerComponent;
    }
}

export async function loadAllIcons() {
    const loadImages = [
        ["bg32", bg],
        ["cursor", cursor],
        ["move_cursor", moveCursor],
        ["crew", crew],
        ["power_grid", power_grid],
        ["conveyor", conveyor],
        ["atomic_battery", atomicBattery],
        ["battery", battery],
        ["excavator", excavator],
        ["storage", storage],
        ["medium_storage", mediumStorage],
        ["crew_cabin", crewCabin],
        ["assembler", assembler],
        ["furnace", furnace],
        ["raw_ore", rawOre],
        ["iron_ingot", ironIngot],
        ["copper_ingot", copperIngot],
        ["lithium_ingot", lithiumIngot],
        ["cilicate", cilicate],
        ["gear", gear],
        ["wire", wire],
        ["circuit", circuit],
        ["battery_item", batteryItem],
        ["construction", construction],
        ["deconstruction", deconstruction],
        ["cleanup", cleanup],
    ].map(async ([name, src]) => {
        return [name, src, await loadImage(src)];
    });
    return Promise.all(loadImages);
}

async function loadImage(url) {
    return new Promise((r) => {
        const i = new Image();
        i.onload = (() => r(i));
        i.src = url;
    });
}

export function formatRecipe(recipe) {
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

export function iconWithCount(itemUrl, count) {
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

export function iconWithoutCount(itemUrl) {
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

export function formatInventory(inventory) {
    let result = "";
    for (let [input, count] of inventory.entries()) {
        const icon = iconWithCount(itemToIcon(input), count);
        if (result) result += " " + icon;
        else result += icon;
    }
    return result;
}

export function formatCrews(building) {
    return `${building.crews} / ${building.max_crews}`;
}

export function formatConstruction(construction) {
    let result = `Type: ${construction.type_}`;
    for (let [input, count] of construction.ingredients.entries()) {
        const icon = iconWithCount(itemToIcon(input), count);
        if (result) result += " " + icon;
        else result += icon;
    }
    return result;
}

export function buildingToIcon(building) {
    switch(building){
        case "Battery": return batteryBuilding;
        case "Power": return power;
        case "Excavator": return excavatorItem;
        case "Storage": return storage;
        case "MediumStorage": return mediumStorage;
        case "CrewCabin": return crewCabin;
        case "Assembler": return assemblerComponent;
        case "Furnace": return furnaceItem;
    }
}
