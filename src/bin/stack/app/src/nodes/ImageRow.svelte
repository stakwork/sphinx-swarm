<script>
  import {
    StructuredListRow,
    StructuredListCell,
    StructuredListInput,
  } from "carbon-components-svelte";
  import CheckmarkFilled from "carbon-icons-svelte/lib/CheckmarkFilled.svelte";

  export let item = {};
  export let repo = "";;
  import moment from "moment"

</script>

<StructuredListRow label for="row-{item.id}">
    <StructuredListCell>{repo}@{item.name}</StructuredListCell>
    <StructuredListCell
      >{moment(item.last_updated).fromNow()}</StructuredListCell
    >
    <StructuredListCell>
      {item.status}
    </StructuredListCell>
    <StructuredListInput
      id="row-{item.name}"
      value="row-{item.name}-value"
      title="row-{item.name}-title"
      name="row-{item.name}-name"
    />
    <StructuredListCell>
      <CheckmarkFilled
        class="bx--structured-list-svg"
        aria-label="select an option"
        title="select an option"
      />
    </StructuredListCell>
  </StructuredListRow>
