<script lang="ts">
  import { onDestroy } from "svelte";
  import { flip } from "svelte/animate";
  import { fade } from "svelte/transition";
  import {
    buildSnapshots,
    describeAction,
    describeData,
    type ActionType,
    type Direction,
    type Snapshot,
    type TransformationAction
  } from "./lib/fp";

  type ApiMode = "filter" | "map" | "chain" | "state";

  type ApiChainOperation = {
    op_type: string;
    param: string;
  };

  type ApiStateMutation = {
    mutation_type: string;
    value: number;
  };

  type ApiTimelineStep = {
    operation: string;
    description: string;
    input?: number[];
    output?: number[];
    duration_ms: number;
  };

  type ApiResponse = {
    steps?: ApiTimelineStep[];
    initial_data?: number[];
    final_result?: number[];
    total_duration_ms?: number;
    [key: string]: unknown;
  };

  const DEFAULT_DATA = `[
  {"id": 1, "team": "alpha", "score": 12, "active": true, "meta": {"region": "north"}},
  {"id": 2, "team": "beta", "score": 7, "active": false, "meta": {"region": "east"}},
  {"id": 3, "team": "alpha", "score": 19, "active": true, "meta": {"region": "north"}},
  {"id": 4, "team": "gamma", "score": 7, "active": true, "meta": {"region": "south"}},
  {"id": 5, "team": "beta", "score": 16, "active": true, "meta": {"region": "east"}}
]`;

  const API_PATHS: Record<ApiMode, string> = {
    filter: "/api/functional/demo/filter",
    map: "/api/functional/demo/map",
    chain: "/api/functional/demo/chain",
    state: "/api/functional/demo/state-transitions"
  };

  function uid(): string {
    return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
  }

  function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }

  function isArrayOfObjects(value: unknown): value is Record<string, unknown>[] {
    return Array.isArray(value) && value.every((item) => isRecord(item));
  }

  function formatJson(value: unknown): string {
    return JSON.stringify(value, null, 2);
  }

  function preview(value: unknown): string {
    if (value === undefined) return "undefined";
    if (typeof value === "string") return value;
    const stringified = JSON.stringify(value);
    if (!stringified) return String(value);
    return stringified.length > 80 ? `${stringified.slice(0, 80)}...` : stringified;
  }

  function parseNumberArray(raw: string): number[] {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      throw new Error("Expected a JSON array of numbers.");
    }
    if (!parsed.every((item) => typeof item === "number" && Number.isFinite(item))) {
      throw new Error("Array must contain only finite numbers.");
    }
    return parsed;
  }

  function parseInteger(value: string, label: string): number {
    const parsed = Number.parseInt(value, 10);
    if (!Number.isFinite(parsed)) {
      throw new Error(`${label} must be a valid integer.`);
    }
    return parsed;
  }

  function columnsFor(data: unknown): string[] {
    if (!isArrayOfObjects(data)) {
      return [];
    }
    const keys = new Set<string>();
    data.slice(0, 12).forEach((item) => {
      Object.keys(item).forEach((k) => keys.add(k));
    });
    return [...keys].slice(0, 6);
  }

  function rowKey(row: unknown, index: number): string {
    if (isRecord(row) && typeof row.id !== "undefined") {
      return `id:${String(row.id)}`;
    }
    return `${index}:${preview(row)}`;
  }

  let sourceJson = DEFAULT_DATA;
  let sourceData: unknown = JSON.parse(DEFAULT_DATA);
  let dataError = "";

  let actions: TransformationAction[] = [
    {
      id: uid(),
      type: "filterGreaterThan",
      field: "score",
      value: "",
      amount: 10,
      direction: "asc"
    },
    {
      id: uid(),
      type: "sortBy",
      field: "score",
      value: "",
      amount: 0,
      direction: "desc"
    }
  ];

  let snapshots: Snapshot[] = buildSnapshots(sourceData, actions);
  let currentStep = 0;
  let isPlaying = false;
  let playbackMs = 900;
  let playbackTimer: ReturnType<typeof setInterval> | null = null;

  let actionType: ActionType = "filterGreaterThan";
  let actionField = "score";
  let actionValue = "10";
  let actionAmount = 10;
  let actionDirection: Direction = "asc";

  let apiBaseUrl = "";
  let apiMode: ApiMode = "chain";
  let numericInput = "[1,2,3,4,5,6,7,8,9,10]";
  let filterCondition = "even";
  let mapTransformation = "double";
  let chainOperations: ApiChainOperation[] = [
    { op_type: "filter", param: "even" },
    { op_type: "map", param: "double" },
    { op_type: "take", param: "3" }
  ];
  let stateInitialValue = 10;
  let stateMutations: ApiStateMutation[] = [
    { mutation_type: "increment", value: 5 },
    { mutation_type: "multiply", value: 2 }
  ];
  let apiRequestPreview = "{}";
  let apiResponseRaw = "{}";
  let apiTimeline: ApiTimelineStep[] = [];
  let apiError = "";
  let apiLoading = false;

  $: currentSnapshot = snapshots[currentStep] ?? snapshots[0];
  $: progress =
    snapshots.length > 1 ? Math.round((currentStep / (snapshots.length - 1)) * 100) : 0;
  $: currentColumns = columnsFor(currentSnapshot?.data);
  $: showFieldInput = ["filterEquals", "filterGreaterThan", "mapMultiply", "sortBy", "groupBy", "distinctBy"].includes(actionType);
  $: showValueInput = actionType === "filterEquals";
  $: showAmountInput = ["filterGreaterThan", "mapMultiply", "take", "skip"].includes(actionType);
  $: showDirectionInput = actionType === "sortBy";

  function resetSnapshots(toStart = true): void {
    try {
      snapshots = buildSnapshots(sourceData, actions);
      dataError = "";
      if (toStart) {
        currentStep = 0;
      } else {
        currentStep = Math.min(currentStep, snapshots.length - 1);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to build snapshots.";
      dataError = message;
      snapshots = [{ label: "Error", description: message, data: sourceData }];
      currentStep = 0;
    }
  }

  function loadSourceData(): void {
    try {
      sourceData = JSON.parse(sourceJson);
      stopPlayback();
      resetSnapshots(true);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Invalid JSON input.";
      dataError = message;
    }
  }

  function addAction(): void {
    if (showFieldInput && !actionField.trim()) {
      dataError = "This action requires a field.";
      return;
    }
    if (actionType === "groupBy" && !actionField.trim()) {
      dataError = "groupBy requires a field.";
      return;
    }

    const action: TransformationAction = {
      id: uid(),
      type: actionType,
      field: actionField.trim(),
      value: actionValue.trim(),
      amount: Number.isFinite(actionAmount) ? actionAmount : 0,
      direction: actionDirection
    };

    actions = [...actions, action];
    stopPlayback();
    resetSnapshots(true);
  }

  function removeAction(id: string): void {
    actions = actions.filter((action) => action.id !== id);
    stopPlayback();
    resetSnapshots(false);
  }

  function moveAction(index: number, direction: -1 | 1): void {
    const target = index + direction;
    if (target < 0 || target >= actions.length) {
      return;
    }
    const next = [...actions];
    [next[index], next[target]] = [next[target], next[index]];
    actions = next;
    stopPlayback();
    resetSnapshots(true);
  }

  function clearActions(): void {
    actions = [];
    stopPlayback();
    resetSnapshots(true);
  }

  function stepTo(index: number): void {
    currentStep = index;
  }

  function stepForward(): void {
    if (currentStep < snapshots.length - 1) {
      currentStep += 1;
    } else {
      stopPlayback();
    }
  }

  function stepBack(): void {
    if (currentStep > 0) {
      currentStep -= 1;
    }
  }

  function stopPlayback(): void {
    if (playbackTimer !== null) {
      clearInterval(playbackTimer);
      playbackTimer = null;
    }
    isPlaying = false;
  }

  function playAll(): void {
    if (snapshots.length <= 1) {
      return;
    }
    if (currentStep >= snapshots.length - 1) {
      currentStep = 0;
    }
    stopPlayback();
    isPlaying = true;
    playbackTimer = setInterval(() => {
      if (currentStep >= snapshots.length - 1) {
        stopPlayback();
      } else {
        currentStep += 1;
      }
    }, Math.max(250, playbackMs));
  }

  function resetPlayback(): void {
    stopPlayback();
    currentStep = 0;
  }

  function addChainOperation(): void {
    chainOperations = [...chainOperations, { op_type: "filter", param: "even" }];
  }

  function removeChainOperation(index: number): void {
    chainOperations = chainOperations.filter((_, i) => i !== index);
  }

  function addStateMutation(): void {
    stateMutations = [...stateMutations, { mutation_type: "increment", value: 1 }];
  }

  function removeStateMutation(index: number): void {
    stateMutations = stateMutations.filter((_, i) => i !== index);
  }

  function normalizedApiBase(): string {
    const raw = apiBaseUrl.trim();
    if (raw) {
      return raw.replace(/\/+$/, "");
    }
    if (typeof window !== "undefined") {
      return window.location.origin;
    }
    return "http://localhost:8000";
  }

  function buildApiPayload(): { endpoint: string; body: unknown } {
    if (apiMode === "filter") {
      return {
        endpoint: API_PATHS.filter,
        body: {
          data: parseNumberArray(numericInput),
          condition: filterCondition
        }
      };
    }
    if (apiMode === "map") {
      return {
        endpoint: API_PATHS.map,
        body: {
          data: parseNumberArray(numericInput),
          transformation: mapTransformation
        }
      };
    }
    if (apiMode === "chain") {
      return {
        endpoint: API_PATHS.chain,
        body: {
          data: parseNumberArray(numericInput),
          operations: chainOperations
        }
      };
    }
    return {
      endpoint: API_PATHS.state,
      body: {
        initial_value: stateInitialValue,
        transitions: stateMutations.map((m) => ({
          mutation_type: m.mutation_type,
          value: parseInteger(String(m.value), "Mutation value")
        }))
      }
    };
  }

  async function runApiDemo(): Promise<void> {
    apiLoading = true;
    apiError = "";
    apiTimeline = [];

    try {
      const payload = buildApiPayload();
      apiRequestPreview = formatJson(payload.body);

      const response = await fetch(`${normalizedApiBase()}${payload.endpoint}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify(payload.body)
      });

      const raw = await response.text();
      let parsed: ApiResponse;
      try {
        parsed = raw ? (JSON.parse(raw) as ApiResponse) : {};
      } catch {
        parsed = { raw_response: raw };
      }

      apiResponseRaw = formatJson(parsed);
      if (!response.ok) {
        apiError = `Request failed with status ${response.status}.`;
        return;
      }

      apiTimeline = Array.isArray(parsed.steps) ? parsed.steps : [];
    } catch (error) {
      const message = error instanceof Error ? error.message : "Request failed.";
      apiError = message;
      apiResponseRaw = formatJson({ error: message });
    } finally {
      apiLoading = false;
    }
  }

  onDestroy(() => {
    stopPlayback();
  });
</script>

<main class="page">
  <header class="hero">
    <p class="kicker">Bun + Svelte + TypeScript</p>
    <h1>Interactive FP Data Playground</h1>
    <p>
      Queue transformations, play them step-by-step, and visualize how user data changes at each
      stage. Backend API demos remain available for the project functional endpoints.
    </p>
  </header>

  <section class="layout">
    <article class="panel panel-large">
      <div class="panel-header">
        <h2>Data Transformation Studio</h2>
        <p>Use your own JSON and make transformations intuitive with action playback.</p>
      </div>

      <div class="input-grid">
        <div>
          <label for="sourceJson">Input JSON</label>
          <textarea id="sourceJson" rows="11" bind:value={sourceJson}></textarea>
          <div class="button-row">
            <button on:click={loadSourceData}>Load Data</button>
            <button class="ghost" on:click={() => (sourceJson = DEFAULT_DATA)}>Reset Sample</button>
          </div>
        </div>

        <div class="composer">
          <h3>Add Action</h3>
          <label for="actionType">Action</label>
          <select id="actionType" bind:value={actionType}>
            <option value="filterEquals">filterEquals</option>
            <option value="filterGreaterThan">filterGreaterThan</option>
            <option value="mapMultiply">mapMultiply</option>
            <option value="sortBy">sortBy</option>
            <option value="groupBy">groupBy</option>
            <option value="take">take</option>
            <option value="skip">skip</option>
            <option value="distinctBy">distinctBy</option>
          </select>

          {#if showFieldInput}
            <label for="actionField">Field Path</label>
            <input id="actionField" type="text" placeholder="score, meta.region" bind:value={actionField} />
          {/if}

          {#if showValueInput}
            <label for="actionValue">Value</label>
            <input id="actionValue" type="text" placeholder='10 or "alpha" or true' bind:value={actionValue} />
          {/if}

          {#if showAmountInput}
            <label for="actionAmount">Amount</label>
            <input id="actionAmount" type="number" bind:value={actionAmount} />
          {/if}

          {#if showDirectionInput}
            <label for="actionDirection">Direction</label>
            <select id="actionDirection" bind:value={actionDirection}>
              <option value="asc">asc</option>
              <option value="desc">desc</option>
            </select>
          {/if}

          <button on:click={addAction}>Queue Action</button>
        </div>
      </div>

      {#if dataError}
        <p class="error">{dataError}</p>
      {/if}

      <div class="queue">
        <div class="panel-header tight">
          <h3>Action Queue ({actions.length})</h3>
          <button class="ghost small" on:click={clearActions} disabled={actions.length === 0}>Clear</button>
        </div>
        {#if actions.length === 0}
          <p class="muted">No actions queued. Add one to build a pipeline.</p>
        {:else}
          <ul>
            {#each actions as action, i (action.id)}
              <li transition:fade={{ duration: 220 }}>
                <span>{i + 1}. {describeAction(action)}</span>
                <div class="button-row">
                  <button class="ghost small" on:click={() => moveAction(i, -1)} disabled={i === 0}>Up</button>
                  <button class="ghost small" on:click={() => moveAction(i, 1)} disabled={i === actions.length - 1}>Down</button>
                  <button class="danger small" on:click={() => removeAction(action.id)}>Remove</button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="playback">
        <div class="panel-header tight">
          <h3>Step Playback</h3>
          <p>{currentStep}/{Math.max(0, snapshots.length - 1)} | {describeData(currentSnapshot?.data)}</p>
        </div>
        <div class="progress-wrap">
          <div class="progress-bar" style={`width: ${progress}%`}></div>
        </div>
        <div class="button-row">
          <button class="ghost" on:click={stepBack} disabled={currentStep === 0}>Back</button>
          <button class="ghost" on:click={stepForward} disabled={currentStep >= snapshots.length - 1}>Step</button>
          <button on:click={playAll} disabled={snapshots.length <= 1 || isPlaying}>Play</button>
          <button class="ghost" on:click={stopPlayback} disabled={!isPlaying}>Pause</button>
          <button class="ghost" on:click={resetPlayback}>Reset</button>
        </div>
        <label for="playbackMs">Play Delay (ms)</label>
        <input id="playbackMs" type="number" min="250" step="50" bind:value={playbackMs} />
      </div>

      <div class="visual-grid">
        <section class="board">
          <div class="panel-header tight">
            <h3>Data Board: {currentSnapshot?.label}</h3>
            <p>{currentSnapshot?.description}</p>
          </div>

          {#if Array.isArray(currentSnapshot?.data)}
            {#if isArrayOfObjects(currentSnapshot.data)}
              <div class="table-wrap">
                <table>
                  <thead>
                    <tr>
                      {#each currentColumns as col}
                        <th>{col}</th>
                      {/each}
                    </tr>
                  </thead>
                  <tbody>
                    {#each (currentSnapshot.data as unknown[]).slice(0, 20) as row, idx (rowKey(row, idx))}
                      <tr animate:flip>
                        {#each currentColumns as col}
                          <td>{preview((row as Record<string, unknown>)[col])}</td>
                        {/each}
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else}
              <div class="chips">
                {#each (currentSnapshot.data as unknown[]).slice(0, 40) as item, idx (rowKey(item, idx))}
                  <span class="chip" animate:flip>{preview(item)}</span>
                {/each}
              </div>
            {/if}
          {:else if isRecord(currentSnapshot?.data)}
            <div class="group-grid">
              {#each Object.entries(currentSnapshot.data).slice(0, 24) as [key, group] (key)}
                <div class="group-card" transition:fade={{ duration: 170 }}>
                  <h4>{key}</h4>
                  <p>{Array.isArray(group) ? `${group.length} item(s)` : "value"}</p>
                  <code>{preview(group)}</code>
                </div>
              {/each}
            </div>
          {:else}
            <pre>{formatJson(currentSnapshot?.data)}</pre>
          {/if}
        </section>

        <section class="timeline">
          <div class="panel-header tight">
            <h3>Transformation Timeline</h3>
            <p>Jump to any step.</p>
          </div>
          <div class="timeline-list">
            {#each snapshots as snapshot, idx}
              <button
                class:active={idx === currentStep}
                class="timeline-item"
                on:click={() => stepTo(idx)}
              >
                <strong>{idx}. {snapshot.label}</strong>
                <span>{snapshot.description}</span>
              </button>
            {/each}
          </div>
          <h4>Current Snapshot JSON</h4>
          <pre>{formatJson(currentSnapshot?.data)}</pre>
        </section>
      </div>
    </article>

    <article class="panel">
      <div class="panel-header">
        <h2>Backend API Runner</h2>
        <p>Exercise `/api/functional/*` endpoints with quick payload controls.</p>
      </div>

      <label for="apiBaseUrl">API Base URL</label>
      <input id="apiBaseUrl" type="text" bind:value={apiBaseUrl} placeholder={typeof window !== "undefined" ? window.location.origin : "http://localhost:8000"} />

      <label for="apiMode">Demo</label>
      <select id="apiMode" bind:value={apiMode}>
        <option value="filter">filter</option>
        <option value="map">map</option>
        <option value="chain">chain</option>
        <option value="state">state-transitions</option>
      </select>

      {#if apiMode !== "state"}
        <label for="numericInput">Numeric Data (JSON array)</label>
        <textarea id="numericInput" rows="4" bind:value={numericInput}></textarea>
      {/if}

      {#if apiMode === "filter"}
        <label for="filterCondition">Condition</label>
        <select id="filterCondition" bind:value={filterCondition}>
          <option value="even">even</option>
          <option value="odd">odd</option>
          <option value="greater_than_5">greater_than_5</option>
          <option value="less_than_10">less_than_10</option>
        </select>
      {/if}

      {#if apiMode === "map"}
        <label for="mapTransformation">Transformation</label>
        <select id="mapTransformation" bind:value={mapTransformation}>
          <option value="double">double</option>
          <option value="square">square</option>
          <option value="increment">increment</option>
          <option value="decrement">decrement</option>
          <option value="absolute">absolute</option>
        </select>
      {/if}

      {#if apiMode === "chain"}
        <div class="panel-header tight">
          <h3>Chain Ops</h3>
          <button class="ghost small" on:click={addChainOperation}>Add</button>
        </div>
        <div class="stack">
          {#each chainOperations as operation, index}
            <div class="inline-row">
              <select bind:value={operation.op_type}>
                <option value="filter">filter</option>
                <option value="map">map</option>
                <option value="take">take</option>
                <option value="skip">skip</option>
              </select>
              <input type="text" bind:value={operation.param} />
              <button class="danger small" on:click={() => removeChainOperation(index)}>x</button>
            </div>
          {/each}
        </div>
      {/if}

      {#if apiMode === "state"}
        <label for="stateInitialValue">Initial Value</label>
        <input id="stateInitialValue" type="number" bind:value={stateInitialValue} />
        <div class="panel-header tight">
          <h3>Mutations</h3>
          <button class="ghost small" on:click={addStateMutation}>Add</button>
        </div>
        <div class="stack">
          {#each stateMutations as mutation, index}
            <div class="inline-row">
              <select bind:value={mutation.mutation_type}>
                <option value="increment">increment</option>
                <option value="decrement">decrement</option>
                <option value="multiply">multiply</option>
                <option value="divide">divide</option>
                <option value="set">set</option>
              </select>
              <input type="number" bind:value={mutation.value} />
              <button class="danger small" on:click={() => removeStateMutation(index)}>x</button>
            </div>
          {/each}
        </div>
      {/if}

      <button on:click={runApiDemo} disabled={apiLoading}>
        {apiLoading ? "Running..." : "Run API Demo"}
      </button>

      {#if apiError}
        <p class="error">{apiError}</p>
      {/if}

      <h3>Request</h3>
      <pre>{apiRequestPreview}</pre>

      <h3>Timeline</h3>
      <div class="stack">
        {#if apiTimeline.length === 0}
          <p class="muted">No step timeline yet.</p>
        {:else}
          {#each apiTimeline as step, idx}
            <div class="api-step" transition:fade={{ duration: 200 }}>
              <strong>{idx + 1}. {step.operation}</strong>
              <p>{step.description}</p>
              <code>in: {formatJson(step.input ?? null)}</code>
              <code>out: {formatJson(step.output ?? null)}</code>
              <small>{step.duration_ms} ms</small>
            </div>
          {/each}
        {/if}
      </div>

      <h3>Raw Response</h3>
      <pre>{apiResponseRaw}</pre>
    </article>
  </section>
</main>
