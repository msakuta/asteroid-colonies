<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    import debug from '../images/ore-overlay.png';
</script>


<img on:click={() => dispatch('click')} src={debug} alt="debug">

<style>
    img {
        position: absolute;
        right: 5px;
        bottom: 48px;
        width: 32px;
        height: 32px;
    }
</style>