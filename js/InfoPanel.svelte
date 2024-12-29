<script>
    import RecipeItem from './RecipeItem.svelte';
    import Inventory from './Inventory.svelte';
    import Construction from './Construction.svelte';
    import { formatCrews } from './graphics';
    export let result;

    let buildingType = "";
    let task = "";
    let recipe = null;
    let oreAccum = null;
    let inventory = new Map();
    let crews = "-";
    let construction = null;
    let extra = "";
    let is_storage = false;
    $: {
        let building = result?.building;
        if (building) {
            buildingType = building.type_;
            task = building.task;
            recipe = building.recipe;
            inventory = building.inventory;
            crews = formatCrews(building);
            if (building.ores){
                console.log(`building.ores.is_storage: ${building.ores.is_storage}`);
                is_storage = building.ores.is_storage;
            }
            oreAccum = building.ores;
        }
        else {
            buildingType = "";
            task = "";
            recipe = null;
            inventory = new Map();
            crews = "-";
            oreAccum = null;
        }
        construction = result?.construction;

        // Time scale = 360
        // 1 energy unit = 360 kJ = 0.36MJ
        extra = result ? `Accumulated energy: ${(result.energy * 0.36).toFixed(2)} MJ
Power capacity: ${result.power_capacity} kW
Power demand: ${result.power_demand} kW
Power load: ${(result.power_demand / result.power_capacity * 100).toFixed(1)} %
Transports: ${result.transports}` : "";
    }

    function barWidth(ore) {
        if(is_storage) {
            return ore;
        }
        else {
            return ore * 100;
        }
    }

    function formatOre(ore) {
        if(is_storage) {
            return ore.toFixed(2);
        }
        else {
            return `${(ore * 100).toFixed(0)}%`;
        }
    }
</script>

<div class="bottomPanel">
    <div class="header">Info Panel</div>
    <pre class="infoPanel">
Building: {buildingType}
Task: {task}
Recipe: {#if recipe}
<RecipeItem item={recipe}/>
{:else}
None
{/if}
</pre>
{#if oreAccum}
<div style="font-family: monospace">
Ores:<br>
&nbsp;Cilicate: <span class="barBackground" style="" >
    <span class="bar" style="width: {barWidth(oreAccum.ores.cilicate)}px" />
    <span class="barText">{formatOre(oreAccum.ores.cilicate)}</span>
  </span><br>
&nbsp;Iron: <span class="barBackground">
    <span class="bar" style="width: {barWidth(oreAccum.ores.iron)}px" />
    <span class="barText">{formatOre(oreAccum.ores.iron)}</span>
  </span><br>
&nbsp;Copper: <span class="barBackground">
    <span class="bar" style="width: {barWidth(oreAccum.ores.copper)}px" />
    <span class="barText">{formatOre(oreAccum.ores.copper)}</span>
  </span><br>
&nbsp;Lithium: <span class="barBackground">
    <span class="bar" style="width: {barWidth(oreAccum.ores.lithium)}px" />
    <span class="barText">{formatOre(oreAccum.ores.lithium)}</span>
  </span><br>
</div>
{/if}
<pre>
Inventory: <Inventory items={inventory} />
Crews: {crews}
Construction: {#if construction}
<Construction {construction}/>
{/if}
{extra}
  </pre>
</div>

<style>
    .bottomPanel {
      position: absolute;
      left: 10px;
      bottom: 10px;
      border: 3px outset #7D7D7D;
      padding: 5px;
      background-color: #afafaf;
    }

    .header {
        font-weight: bold;
    }

    .infoPanel {
        margin: 0;
    }

    .barBackground {
        position: relative;
        display: inline-block;
        left: 0px;
        top: 0px;
        height: 1em;
        width: 100px;
        background-color: #000;
    }

    .barText {
        position: absolute;
        left: 0px;
        top: 0px;
        width: 100px;
        text-align: center;
        color: #fff;
    }

    .bar {
        position: absolute;
        display: block;
        left: 0px;
        top: 0px;
        height: 1em;
        background-color: #007f00;
    }
</style>
