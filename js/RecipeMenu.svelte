<script>
    import { createEventDispatcher } from 'svelte';
    import IconWithCount from "./IconWithCount.svelte";
    import CloseButton from './CloseButton.svelte';
    import { itemToIcon } from "./graphics";

    const dispatch = createEventDispatcher();

    export let items = [];
</script>

<div class="recipes">
    <CloseButton on:close={() => dispatch('close')}/>
    <div>Select a recipe</div>
    <div>
        <div class="recipe" on:click={() => dispatch('clear')}>No Recipe</div>
    </div>
    {#each items as item}
    <div class="recipe" on:click={() => dispatch('click', {type: item.outputs.keys().next().value})}>
        <IconWithCount itemUrl={itemToIcon(item.type_)} />
        &lt;=
        {#each item.inputs as [input, count]}
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
</style>
