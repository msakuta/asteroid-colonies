<script>
    import { createEventDispatcher } from 'svelte';
    import radialMenu from '../images/radialMenu.png';
    import excavate from '../images/excavate.png';
    import moveBuilding from '../images/moveBuilding.png';
    import build from '../images/build.png';

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
        ["50%", "20%"],
        ["20%", "50%"],
        ["80%", "50%"],
        ["50%", "80%"],
    ];
</script>

<div class="background" on:pointerup={() => dispatch('close')}>
    <div class="radialMenu noselect" style="background-image: url({radialMenu}); transform: translate({pos[0] - menuWidth / 2}px, {pos[1] - menuWidth / 2}px)">
        {#if centerIcon}
        <div class="icon centerIcon" style="background-image: url({centerIcon})"/>
        {/if}
        {#each items.map((e, i) => [e, itemPositions[i]]) as [item, pos]}
        <div class="itemContainer" style="left: {pos[0]}; top: {pos[1]}" on:pointerup|stopPropagation={() => dispatch(item.event)}>
            <div class="icon" style="background-image: url({item.icon});"></div>
            {item.caption}
        </div>
        {/each}
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
        background-size: 256px 256px;
    }

    .itemContainer {
        position: absolute;
        left: 50%;
        top: 20%;
        transform: translate(-50%, -50%);
        color: #fff;
        text-align: center;
        font-size: 10pt;
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
