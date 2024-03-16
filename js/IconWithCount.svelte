<script>
    export let itemUrl;
    export let count = null;
    export const widthFactor = 1;
    export const heightFactor = 1;

</script>

<div class="item" style="
background-image: url({itemUrl});
background-size: {32 * widthFactor}px {32 * heightFactor}px;
">
{#if count !== null}
    <div class="overlay noselect">
    {count}
    </div>
{/if}
</div>

<style>
    div {
        display: inline-block;
        position: relative;
        width: 32px;
        height: 32px;
    }

    .overlay{
      position: absolute;
      left: 0px;
      bottom: 0px;
      width: 100%;
      height: auto;
      color: rgb(0,0,0);
      font-family: Monospace;
      font-weight: bold;
      text-shadow: 1px 1px #fff, -1px -1px 0 #fff, 1px -1px 0 #fff, -1px 1px 0 #fff;
      text-align: right;
      pointer-events:none;
      z-index: 100;
    }
</style>
