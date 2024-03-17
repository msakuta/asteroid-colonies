<script>
    import buttonFrame from '../images/buttonFrame.png';
    import buttonFrameActive from '../images/buttonFrameActive.png';

    export let modeName = "";
    export let buttons = [];
    let frameImage = (modeName, button) => button.mode === modeName ? buttonFrameActive : buttonFrame;

    function clickButton(button) {
        if (modeName === button.mode) {
            modeName = "";
        }
        else {
            modeName = button.mode;
        }
    }
</script>

<div>
    {#each buttons as button, idx}
    <div class="buttonContainer" style="top: {idx * 48}px;
                background-image: url({frameImage(modeName, button)})">
        <img src={button.icon} alt="button" on:pointerup={() => clickButton(button)}>
    </div>
    {/each}
</div>

<style>
    .buttonContainer {
        position: absolute;
        right: 0;
        width: 48px;
        height: 48px;
        background-size: 48px 48px;
    }

    img {
        position: absolute;
        left: 50%;
        top: 50%;
        transform: translate(-50%, -50%);
    }
</style>
