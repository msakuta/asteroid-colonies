<script>
    import RecipeItem from './RecipeItem.svelte';
    import Inventory from './Inventory.svelte';
    import Construction from './Construction.svelte';
    import Ores from './Ores.svelte';
    import { formatCrews } from './graphics';
    export let result;

    let buildingType = "";
    let task = "";
    let recipe = null;
    let oreAccum = null;
    let countableInventory = new Map();
    let oresInventory = null;
    let crews = "-";
    let construction = null;
    let extra = "";
    let ores = null;
    $: {
        let building = result?.building;
        if (building) {
            buildingType = building.type_;
            task = building.task;
            recipe = building.recipe;
            countableInventory = building.inventory.countable;
            oresInventory = building.inventory.ores;
            crews = formatCrews(building);
            oreAccum = building.ores;
        }
        else {
            buildingType = "";
            task = "";
            recipe = null;
            countableInventory = new Map();
            oresInventory = null;
            crews = "-";
            oreAccum = null;
        }
        construction = result?.construction;
        ores = result?.ores;

        // Time scale = 360
        // 1 energy unit = 360 kJ = 0.36MJ
        extra = result ? `Accumulated energy: ${(result.energy * 0.36).toFixed(2)} MJ
Power capacity: ${result.power_capacity} kW
Power demand: ${result.power_demand} kW
Power load: ${(result.power_demand / result.power_capacity * 100).toFixed(1)} %
Transports: ${result.transports}` : "";
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
<Ores ores={oreAccum} title="Smelting ores:"/>
{/if}
<tt>
Inventory: <Inventory items={countableInventory} />
</tt>
{#if oresInventory}
<Ores ores={oresInventory} isStorage={true} title="Inventory ores:"/>
{/if}
<pre>
Crews: {crews}
Construction: {#if construction}
<Construction {construction}/>
{/if}
</pre>
{#if ores}
<Ores ores={ores} title="Ores:"/>
{/if}
<pre>
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
</style>
