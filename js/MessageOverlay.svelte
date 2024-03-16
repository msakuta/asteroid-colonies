<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    export let text = "";
    export let showOkButton = false;
    export let showCancelButton = false;
</script>

<div class="messageOverlay" style="display: block">
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

<style>
    .messageOverlay{
        position: absolute;
        left: 0;
        right: 0;
        top: 0px;
        height: auto;
        color: rgb(255,255,255);
        background-color: rgba(0, 0, 0, 0.75);
        font-weight: bold;
        text-shadow: 1px 1px #000, -1px -1px 0 #000, 1px -1px 0 #000, -1px 1px 0 #000;
        text-align: center;
        /* pointer-events:none; */
        z-index: 100;
    }
</style>