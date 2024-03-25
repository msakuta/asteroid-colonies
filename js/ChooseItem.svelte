<script>
    import IconWithCount from "./IconWithCount.svelte";
    import CloseButton from "./CloseButton.svelte";
    import { itemToIcon } from "./graphics";
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let items = [];
</script>

<div class="items">
    <CloseButton on:close={() => dispatch('close')}/>
    <div>Select an item</div>
    <span class="itemsContainer">
        {#each items.entries() as [item, count]}
        <IconWithCount itemUrl={itemToIcon(item)} {count} on:click={() => dispatch('click', item)}/>
        {/each}
    </span>
</div>

<style>
    .items {
        border: 1px solid black;
        background-color: #afafaf;
        padding: 4px;
        z-index: 100;
        position: fixed;
        top: 50%;
        left: 50%;
        min-width: 200px;
        margin-right: -50%;
        transform: translate(-50%, -50%);
    }

    .itemsContainer {
        padding: 10px;
    }
</style>
