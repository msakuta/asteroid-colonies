<script>
    export let ores = null;
    export let isStorage = false;
    export let title = "Ores:";

    let barClass = isStorage ? "bar barGreen" : "bar barYellow";

    function barWidth(ore) {
        if(isStorage) {
            return ore;
        }
        else {
            return ore * 100;
        }
    }

    function formatOre(ore) {
        if(isStorage) {
            return ore.toFixed(2);
        }
        else {
            return `${(ore * 100).toFixed(0)}%`;
        }
    }
</script>


<div style="font-family: monospace">
    {title}<br>
    &nbsp;Cilicate: <span class="barBackground" style="" >
        <span class={barClass} style="width: {barWidth(ores.cilicate)}px" />
        <span class="barText">{formatOre(ores.cilicate)}</span>
      </span><br>
    &nbsp;Iron: <span class="barBackground">
        <span class={barClass} style="width: {barWidth(ores.iron)}px" />
        <span class="barText">{formatOre(ores.iron)}</span>
      </span><br>
    &nbsp;Copper: <span class="barBackground">
        <span class={barClass} style="width: {barWidth(ores.copper)}px" />
        <span class="barText">{formatOre(ores.copper)}</span>
      </span><br>
    &nbsp;Lithium: <span class="barBackground">
        <span class={barClass} style="width: {barWidth(ores.lithium)}px" />
        <span class="barText">{formatOre(ores.lithium)}</span>
      </span><br>
</div>

<style>
    .barBackground {
        position: relative;
        display: inline-block;
        left: 0px;
        top: 0px;
        height: 1em;
        width: 100px;
        background-color: #000;
    }

    .barText {
        position: absolute;
        left: 0px;
        top: 0px;
        width: 100px;
        text-align: center;
        color: #fff;
    }

    .bar {
        position: absolute;
        display: block;
        left: 0px;
        top: 0px;
        height: 1em;
    }

    .barGreen {
        background-color: #007f00;
    }

    .barYellow {
        background-color: #7f7f00;
    }
</style>
