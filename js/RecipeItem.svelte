<script>
    import { createEventDispatcher } from 'svelte';
    import IconWithCount from "./IconWithCount.svelte";
    import { itemToIcon } from "./graphics";

    const dispatch = createEventDispatcher();

    export let item;
    $: type = item.outputs.keys().next().value;
</script>

<div class="recipe" on:pointerup={() => dispatch('click', {type})}>
    <IconWithCount itemUrl={itemToIcon(type)} />
    &lt;=
    {#each item.inputs as [input, count]}
        <IconWithCount itemUrl={itemToIcon(input)} {count}/>
    {/each}
</div>

<style>
    .recipe {
      position: relative;
      margin: 4px;
      padding: 4px;
      border: 1px solid #7f7f3f;
      background-color: #ffff7f;
      white-space: normal;
    }
</style>