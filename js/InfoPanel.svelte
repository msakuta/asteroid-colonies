<script>
    export let result;
    let elem;

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

<div bind:this={elem}>
    Info panel
</div>
