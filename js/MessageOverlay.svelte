<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let text = "";
    export let showOkButton = false;
    export let showCancelButton = false;
</script>

<div id="messageOverlay" class="messageOverlay" style="display: block">
    {text}
    {#if showOkButton || showCancelButton}
    <div>
        {#if showOkButton}
        <button value="Ok" on:click={() => dispatch('ok')}>Ok</button>
        {/if}
        {#if showCancelButton}
        <button value="Cancel" on:click={() => dispatch('cancel')}>Cancel</button>
        {/if}
    </div>
    {/if}
</div>
