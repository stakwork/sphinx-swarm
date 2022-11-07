<script>
</script>

<div class="tool-card">
    <slot/>
</div>

<style>
    .tool-card {
        padding: 10px;
        border-radius: 10px;
        /*display: flex;*/
        align-items: center;
        max-height: 10rem;
        width: calc(25vw - 40px);
        margin: 20px;
        box-shadow: 0 1.5px 3px 0 rgba(211, 211, 211, 0.2), 0 2px 8px 0 rgba(211, 211, 211, 0.19);
    }
</style>