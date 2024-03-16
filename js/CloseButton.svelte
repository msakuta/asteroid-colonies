<script>
    import { createEventDispatcher } from 'svelte';
    import closeButton from '../images/close.png';

    const dispatch = createEventDispatcher();
</script>

<span on:click={() => dispatch('close')} style="background-image: url({closeButton})"/>

<style>
    span {
        position: absolute;
        right: 5px;
        top: 5px;
        width: 16px;
        height: 16px;
    }
</style>
