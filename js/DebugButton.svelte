<script>
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    import debug from '../images/debug.png';
</script>


<img on:click={() => dispatch('click')} src={debug} alt="debug">

<style>
    img {
        position: absolute;
        right: 5px;
        bottom: 10px;
        width: 64px;
        height: 32px;
    }
</style>