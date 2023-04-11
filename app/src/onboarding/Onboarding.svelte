<script lang="ts">
  import { Button, Popover } from "carbon-components-svelte";
  import {
    finishedOnboarding,
    selectedNode,
    stack,
    onChainAddressGeneratedForOnboarding,
    copiedAddressForOnboarding,
    lndBalances,
    unconfirmedBalance,
  } from "../store";
  import * as api from "../api";

  $: tag = "";

  $: currentStep = 0;
  $: $stack, determineCurrentStep(), determineTag();
  $: disabled = true;
  $: $onChainAddressGeneratedForOnboarding, onChainAddressGenerated();
  $: $copiedAddressForOnboarding, copiedAddressHandler();
  let open = true;
  $: currentStep, checkForConfirmedTransaction();
  $: $finishedOnboarding, determineCurrentStep();

  function onChainAddressGenerated() {
    disabled = !$onChainAddressGeneratedForOnboarding;
  }
  function copiedAddressHandler() {
    disabled = !$copiedAddressForOnboarding;
  }

  function checkForConfirmedTransaction() {
    if (currentStep === 2) {
      const interval = setInterval(async () => {
        const balance = await api.lnd.get_balance(tag);
        updateUnconfirmedBalance(balance);
        updateConfirmedBalance(balance);
        if (balance?.confirmed_balance > 0) {
          clearInterval(interval);
          disabled = false;
        }
      }, 60000);
    }
  }

  function determineTag() {
    const lightning = $stack.nodes.find(
      (node) => node.type === "Lnd" || node.type === "Cln"
    );
    if (lightning) {
      tag = lightning.name;
    }
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
      } else if (currentStep === 3) {
        disabled = true;
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
    const hasBalance = $finishedOnboarding.hasBalance;
    if (!hasBalance) {
      const lightning = $stack.nodes.find((node) => node.type === "Lnd");
      if (lightning) {
        selectedNode.update(() => lightning);
      }
      currentStep = 0;
      console.log("We got here");
    } else if (!hasChannels) {
      currentStep = 3;
    } else if (!hasAdmin) {
      currentStep = 3;
    }
  }

  function togglePopover() {
    open = !open;
  }

  function updateConfirmedBalance(balance) {
    if (
      lndBalances.hasOwnProperty(tag) &&
      lndBalances[tag] === balance?.confirmed_balance
    )
      return;
    lndBalances.update((n) => {
      return { ...n, [tag]: balance?.confirmed_balance };
    });
  }

  function updateUnconfirmedBalance(balance) {
    if (
      unconfirmedBalance.hasOwnProperty(tag) &&
      unconfirmedBalance[tag] === balance?.unconfirmed_balance
    )
      return;
    unconfirmedBalance.update((n) => {
      return { ...n, [tag]: balance?.unconfirmed_balance };
    });
  }
</script>

<section class="onboarding_section" style:position="relative">
  {#if !$finishedOnboarding.hasBalance || !$finishedOnboarding.hasChannels || !$finishedOnboarding.hasAdmin}
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
