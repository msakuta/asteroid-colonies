<script>
    import { createEventDispatcher } from 'svelte';
    import excavate from '../images/excavate.png';
    import moveBuilding from '../images/moveBuilding.png';
    import build from '../images/build.png';
    import RadialMenuSvg from './RadialMenuSvg.svelte';

    const dispatch = createEventDispatcher();

    const menuWidth = 256;

    export let centerIcon = null;
    export let pos = [0, 0];
    export let items = [
        {caption: "Excavate", event: 'excavate', icon: excavate},
        {caption: "Move Bldg.", event: 'moveBuilding', icon: moveBuilding},
        {caption: "Build", event: 'build', icon: build},
    ];
    export let itemPositions = [
        ["50%", "15%"],
        ["20%", "35%"],
        ["80%", "35%"],
        ["20%", "65%"],
        ["80%", "65%"],
        ["50%", "85%"],
    ];

    function dispatchFilter(item) {
        if (!item.grayed) {
            dispatch(item.event);
        }
    }

    function radialMenuClick(evt) {
        console.log(`clicked: ${evt.detail}`);
        dispatchFilter(items[evt.detail]);
    }
</script>

<div class="background" on:pointerup={() => dispatch('close')}>
    <div class="radialMenu noselect" style="transform: translate({pos[0] - menuWidth / 2}px, {pos[1] - menuWidth / 2}px)">
        <div class="animContainer">
            {#if centerIcon}
            <div class="icon centerIcon" style="background-image: url({centerIcon})"/>
            {/if}
            {#each items.map((e, i) => [e, itemPositions[i]]) as [item, pos]}
            <div class="itemContainer"
                class:grayed={item.grayed}
                style="left: {pos[0]}; top: {pos[1]}"
            >
                <div class="icon" style="background-image: url({item.icon})"></div>
                {item.caption}
            </div>
            {/each}
            <RadialMenuSvg {items} on:click={radialMenuClick}/>
        </div>
    </div>
</div>

<style>
    .background {
        position: absolute;
        left: 0;
        top: 0;
        right: 0;
        bottom: 0;
        background-color: rgba(0,0,0, 0.25);
    }

    .radialMenu {
        position: absolute;
        left: 0;
        top: 0;
        transform: translate(-50%, -50%);
        width: 256px;
        height: 256px;
    }

    .animContainer {
        position: absolute;
        left: 0;
        top: 0;
        right: 0;
        bottom: 0;
        background-size: 256px 256px;
        animation: 0.15s ease-out 0.075s 1 both running scaleup;
    }

    @keyframes scaleup {
        from {
            transform: scale(0);
        }
        to {
            transform: scale(1);
        }
    }

    .itemContainer {
        position: absolute;
        left: 50%;
        top: 20%;
        width: 64px;
        transform: translate(-50%, -50%);
        color: #fff;
        text-align: center;
        font-size: 10pt;
        pointer-events: none;
    }

    .grayed {
        filter: brightness(0.5)
    }

    .icon {
        margin: auto;
        width: 32px;
        height: 32px;
        background-size: 32px 32px;
    }

    .centerIcon {
        position: absolute;
        left: 50%;
        top: 50%;
        transform: translate(-50%, -50%);
    }
</style>
