<script>
    import { createEventDispatcher } from 'svelte';
    import errorIcon from '../images/errorIcon.png';

    const dispatch = createEventDispatcher();

    export let text = "";
    export let timeout = 3;
    export let maxTimeout = 3;
    $: opacity = Math.min(timeout, 1);
</script>

<div class="messageOverlay noselect" style="display: block; opacity: {opacity}" on:pointerup={() => dispatch('click')}>
    <img src={errorIcon} alt="error">
    {text}
    <div class="timeoutBar" style="width: {timeout / maxTimeout * 100}%"/>
</div>

<style>
    .messageOverlay{
        position: absolute;
        left: 50%;
        top: 20%;
        padding: 0.5em;
        vertical-align: middle;
        transform: translate(-50%, 0);
        height: auto;
        color: rgb(255,255,255);
        background-color: rgba(0, 0, 0, 0.75);
        font-weight: bold;
        text-shadow: 1px 1px #000, -1px -1px 0 #000, 1px -1px 0 #000, -1px 1px 0 #000;
        text-align: center;
        /* pointer-events:none; */
        z-index: 100;
    }

    .timeoutBar {
        position: absolute;
        left: 0;
        bottom: 0;
        background-color: rgb(255, 127, 127);
        height: 5px;
    }
</style>