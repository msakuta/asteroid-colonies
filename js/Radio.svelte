<script>
    export let options;
    export let legend;
    export let userSelected = options[0].value;
    export let fontSize = 16;
    export let flexDirection = 'column';
</script>

<div role="radiogroup" style="display: flex; font-size:{fontSize}px; flex-direction:{flexDirection}">
    <div class="legend">{legend}</div>
    {#each options as { value, label }}
        <label>
            <input
            type="radio"
            bind:group={userSelected}
            value={value} />
            {label}
        </label>
    {/each}
</div>
