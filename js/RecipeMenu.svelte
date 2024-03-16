<script>
    import { createEventDispatcher } from 'svelte';
    import CloseButton from './CloseButton.svelte';
    import RecipeItem from './RecipeItem.svelte';

    const dispatch = createEventDispatcher();

    export let items = [];
</script>

<div class="recipes">
    <CloseButton on:close={() => dispatch('close')}/>
    <div>Select a recipe</div>
    <div>
        <div class="recipe" on:pointerup={() => dispatch('clear')}>No Recipe</div>
    </div>
    {#each items as item}
    <RecipeItem {item} on:click={evt => dispatch('click', evt.detail)}/>
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
