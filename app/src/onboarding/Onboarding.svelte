<script lang="ts">
  import { Button, Popover } from "carbon-components-svelte";
  import {
    finishedOnboarding,
    selectedNode,
    stack,
    onChainAddressGeneratedForOnboarding,
    copiedAddressForOnboarding,
  } from "../store";

  $: currentStep = 0;
  $: $stack, determineCurrentStep();
  $: disabled = true;
  $: $onChainAddressGeneratedForOnboarding, onChainAddressGenerated();
  $: $copiedAddressForOnboarding, copiedAddressHandler();
  let open = true;

  function onChainAddressGenerated() {
    disabled = !$onChainAddressGeneratedForOnboarding;
  }
  function copiedAddressHandler() {
    disabled = !$copiedAddressForOnboarding;
  }

  const steps = [
    { text: "Generate onchain address" },
    { text: "Copy address and send some bitcoin to generated address" },
    { text: "Waiting for transaction to recieve 6 confirmations" },
    { text: "Peer with Sphinx node (Game_b 1)" },
    {
      text: "Create channel with Game_b 1 and send some sats to the other side",
    },
    { text: "Scan qr code to get started on the sphinx app" },
    { text: "Add new users to your Swarm" },
  ];

  const nextOnboardingHandler = () => {
    if (currentStep < steps.length - 1) {
      currentStep += 1;
      if (currentStep === 1 && $copiedAddressForOnboarding) {
        disabled = false;
      } else {
        disabled = true;
      }
    }
  };

  const prevOnboardingHandler = () => {
    if (currentStep > 0) {
      currentStep -= 1;
      disabled = false;
    }
  };

  function determineCurrentStep() {
    const hasAdmin = $finishedOnboarding.hasAdmin;
    const hasChannels = $finishedOnboarding.hasChannels;
    if (!hasChannels) {
      const lightning = $stack.nodes.find((node) => node.type === "Lnd");
      if (lightning) {
        selectedNode.update(() => lightning);
      }
      currentStep = 0;
    } else if (!hasAdmin) {
      currentStep = 3;
    }
  }

  function togglePopover() {
    open = !open;
  }
</script>

<section class="onboarding_section" style:position="relative">
  {#if !$finishedOnboarding.hasAdmin || !$finishedOnboarding.hasChannels}
    <Button
      on:click={togglePopover}
      size="field"
      kind="secondary"
      class="onboarding_btn">Onboarding</Button
    >
    <Popover bind:open align="bottom-left" caret light highContrast>
      <div class="popover_content_container">
        <p>{steps[currentStep].text}</p>
        <div class="button_container">
          {#if currentStep > 0}
            <button type="button" class="btn" on:click={prevOnboardingHandler}
              >Prev</button
            >
          {/if}
          <button
            {disabled}
            type="button"
            class="btn next_btn"
            on:click={nextOnboardingHandler}>Next</button
          >
        </div>
      </div>
    </Popover>
  {/if}
</section>

<style>
  .onboarding_section {
    margin-right: 1rem;
  }

  .popover_content_container {
    padding: 0.8rem;
  }

  .button_container {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.3rem;
  }

  .btn {
    border: none;
    cursor: pointer;
    padding: 0.3rem 1rem;
  }

  .next_btn {
    margin-left: auto;
    color: #212121;
  }
  .next_btn:disabled {
    background-color: none;
    color: #bfbfbf;
  }
</style>
