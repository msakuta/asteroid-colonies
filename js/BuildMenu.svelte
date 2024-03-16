<script>
    import { createEventDispatcher } from 'svelte';
    import IconWithCount from "./IconWithCount.svelte";
    import CloseButton from './CloseButton.svelte';
    import { itemToIcon, buildingToIcon } from "./graphics";

    const dispatch = createEventDispatcher();

    export let items = [];

    function constructionToItem(type) {
        return buildingToIcon(type.Building);
    }
</script>

<div class="recipes">
    <CloseButton on:close={() => dispatch('close')}/>
    <div>Select a building</div>
    {#each items as item}
    <div class="recipe" on:click={() => dispatch('click', {type: item.type_.Building})}>
        <IconWithCount itemUrl={constructionToItem(item.type_)} />
        &lt;=
        {#each item.ingredients as [input, count]}
            <IconWithCount itemUrl={itemToIcon(input)} {count}/>
        {/each}
    </div>
    {/each}
</div>

<style>
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