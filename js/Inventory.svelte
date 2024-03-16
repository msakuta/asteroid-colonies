<script>
    import IconWithCount from "./IconWithCount.svelte";
    import { itemToIcon } from "./graphics";
    export let items = [];
</script>

<span>
    {#each items.entries() as [item, count]}
    <IconWithCount itemUrl={itemToIcon(item)} {count}/>
    {/each}
</span>
