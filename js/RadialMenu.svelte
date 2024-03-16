<script>
    import { createEventDispatcher } from 'svelte';
    import radialMenu from '../images/radialMenu.png';
    import excavate from '../images/excavate.png';
    import moveBuilding from '../images/moveBuilding.png';

    const dispatch = createEventDispatcher();

    const menuWidth = 512;

    export let pos = [0, 0];
</script>

<div class="background" on:pointerup={() => dispatch('close')}>
    <div class="radialMenu noselect" style="background-image: url({radialMenu}); transform: translate({pos[0] - menuWidth / 2}px, {pos[1] - menuWidth / 2}px)">
        <div class="itemContainer" on:pointerup|stopPropagation={() => dispatch('excavate')}>
            <div class="icon" style="background-image: url({excavate})"></div>
            Excavate
        </div>
        <div class="itemContainer" style="left: 20%; top: 50%"
            on:pointerup|stopPropagation={() => dispatch('moveBuilding')}>
            <div class="icon" style="background-image: url({moveBuilding})"></div>
            Move Building
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
        max-width: 75%;
        max-height: 75%;
        width: 512px;
        height: 512px;
        background-size: 512px 512px;
    }

    .itemContainer {
        position: absolute;
        left: 50%;
        top: 20%;
        transform: translate(-50%, -50%);
        color: #fff;
        text-align: center;
    }

    .icon {
        margin: auto;
        width: 64px;
        height: 64px;
        background-size: 64px 64px;
    }
</style>
