<script>
    import IconWithCount from "./IconWithCount.svelte";
    import { itemToIcon } from "./graphics";
    export let construction;

    function formatConstructionType(type) {
      if (typeof type === "string") {
        return type;
      }
      else if(type.Building) {
        return type.Building;
      }
      else {
        return type.Conveyor;
      }
    }
</script>

<span>
Type: {formatConstructionType(construction.type_)}
{#each construction.ingredients.entries() as [input, count]}
    <IconWithCount itemUrl={itemToIcon(input)} {count} />
{/each}
</span>
