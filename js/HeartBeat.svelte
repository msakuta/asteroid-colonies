<script>
    import heart from '../images/heart.png';
    import brokenHeart from '../images/brokenHeart.png';
    export let broken = false;
    export let opacity = 1;
</script>

<div>
    {#if broken}
    <img src={brokenHeart} alt="heart" style="opacity: {opacity}">
    {:else}
    <img src={heart} alt="heart" style="opacity: {opacity}">
    {/if}
</div>
