<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    import debug from '../images/debug.png';
</script>


<img on:click={() => dispatch('click')} src={debug}>

<style>
    img {
        position: absolute;
        right: 0;
        bottom: 0;
        width: 64px;
        height: 32px;
    }
</style>