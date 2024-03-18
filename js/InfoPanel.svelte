<script>
    import RecipeItem from './RecipeItem.svelte';
    import Inventory from './Inventory.svelte';
    import Construction from './Construction.svelte';
    import { formatCrews } from './graphics';
    export let result;

    let buildingType = "";
    let task = "";
    let recipe = null;
    let inventory = new Map();
    let crews = "-";
    let construction = null;
    let extra = "";
    $: {
        let building = result?.building;
        if (building) {
            buildingType = building.type_;
            task = building.task;
            recipe = building.recipe;
            inventory = building.inventory;
            crews = formatCrews(building);
        }
        else {
            buildingType = "";
            task = "";
            recipe = null;
            inventory = new Map();
            crews = "-";
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
</style>
