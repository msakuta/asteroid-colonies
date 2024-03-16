<script>
    import { formatInventory, formatRecipe, formatCrews, formatConstruction } from './graphics';
    export let result;
    let elem;

    // Time scale = 360
    // 1 energy unit = 360 kJ = 0.36MJ
    $: if(elem) elem.innerHTML = result ? `Building: ${result.building?.type_}
    Task: ${result.building?.task}
    Recipe: ${result.building?.recipe ? formatRecipe(result.building.recipe) : ""}
    Inventory: ${result.building?.inventory ? formatInventory(result.building.inventory) : ""}
    Crews: ${result.building ? formatCrews(result.building) : ""}
    Construction: ${result.construction ? formatConstruction(result.construction) : ""}
    Accumulated energy: ${(result.energy * 0.36).toFixed(2)} MJ
    Power capacity: ${result.power_capacity} kW
    Power demand: ${result.power_demand} kW
    Power load: ${(result.power_demand / result.power_capacity * 100).toFixed(1)} %
    Transports: ${result.transports}` : "";

</script>

<div class="bottomPanel">
    <pre class="infoPanel" bind:this={elem}>
        Info panel
    </pre>
</div>

<style>
    .bottomPanel {
      position: absolute;
      left: 10px;
      bottom: 10px;
      border: 1px solid black;
      background-color: #afafaf;
    }

    .infoPanel {
      border: 1px solid black;
      background-color: #afafff;
    }

    .recipes {
      border: 1px solid black;
      background-color: #afafaf;
      padding: 4px;
      z-index: 100;
      position: fixed;
      top: 50%;
      left: 50%;
      margin-right: -50%;
      transform: translate(-50%, -50%);
    }

    .recipe {
      position: relative;
      margin: 4px;
      padding: 4px;
      border: 1px solid #7f7f3f;
      background-color: #ffff7f;
      white-space: normal;
    }
</style>