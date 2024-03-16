<script>
	import Radio from './Radio.svelte'
	
	export let radioValue;
	
	const options = [
        {value: "excavate", label: "Excavate"},
        {value: "move", label: "Move Building"},
        {value: "power", label: "Install PowerGrid"},
        {value: "conveyor", label: "Install Conveyor"},
        {value: "splitter", label: "Install Splitter"},
        {value: "merger", label: "Install Merger"},
        {value: "moveItem", label: "Move Item"},
        {value: "build", label: "Build"},
        {value: "cancel", label: "Cancel"},
        {value: "deconstruct", label: "Deconstruct"},
        {value: "recipe", label: "Set"},
        {value: "cleanup", label: "Cleanup"},
    ]
</script>

<div class="sidePanel">
    <Radio {options} fontSize={16} legend='Click Mode' bind:userSelected={radioValue}/>
    <div>
        {radioValue} is selected
    </div>
</div>

<style>
    .sidePanel {
        position: absolute;
        right: 10px;
        top: 10px;
        border: 1px solid black;
        background-color: #afffaf;
    }
</style>
